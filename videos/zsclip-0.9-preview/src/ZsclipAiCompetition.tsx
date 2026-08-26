import React from "react";
import {
  AbsoluteFill,
  Easing,
  Img,
  Sequence,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { Video } from "@remotion/media";

const FPS = 30;
const TOTAL_SECONDS = 216;
const COVER_INTRO_SECONDS = 3.1;
export const ZSCLIP_AI_TOTAL_FRAMES = TOTAL_SECONDS * FPS;

const seconds = (value: number) => Math.round(value * FPS);

const clampOptions = {
  extrapolateLeft: "clamp" as const,
  extrapolateRight: "clamp" as const,
};

const chapters = [
  {
    start: 0,
    end: 29.31,
    title: "从 Python 小工具到 Rust",
    label: "来路",
    accent: "#22c55e",
  },
  {
    start: 29.31,
    end: 59.35,
    title: "纯原生 UI 与系统工具",
    label: "原生",
    accent: "#38bdf8",
  },
  {
    start: 59.35,
    end: 77.66,
    title: "VV 与复制记录",
    label: "VV",
    accent: "#f97316",
  },
  {
    start: 77.66,
    end: 114,
    title: "组合搜索与右键分组",
    label: "搜索/分组",
    accent: "#a78bfa",
  },
  {
    start: 114,
    end: 133.22,
    title: "OCR 与素材化",
    label: "OCR",
    accent: "#14b8a6",
  },
  {
    start: 133.22,
    end: 165.76,
    title: "0.1M 内存占用美学",
    label: "内存",
    accent: "#facc15",
  },
  {
    start: 165.76,
    end: 186.18,
    title: "ZSUI 开放工程",
    label: "ZSUI",
    accent: "#fb7185",
  },
  {
    start: 186.18,
    end: 216,
    title: "初代多平台测试",
    label: "测试",
    accent: "#60a5fa",
  },
];

type Chapter = (typeof chapters)[number];

const progressChapters = [
  { start: 0, end: 14.09, label: "系统", accent: "#22c55e" },
  { start: 14.09, end: 29.31, label: "Python", accent: "#22c55e" },
  { start: 29.31, end: 42.53, label: "Rust", accent: "#38bdf8" },
  { start: 42.53, end: 59.35, label: "原生", accent: "#38bdf8" },
  { start: 59.35, end: 77.66, label: "VV", accent: "#f97316" },
  { start: 77.66, end: 98.52, label: "搜索", accent: "#a78bfa" },
  { start: 98.52, end: 114, label: "分组", accent: "#a78bfa" },
  { start: 114, end: 133.22, label: "OCR", accent: "#14b8a6" },
  { start: 133.22, end: 148.7, label: "内存", accent: "#facc15" },
  { start: 148.7, end: 165.76, label: "优化", accent: "#facc15" },
  { start: 165.76, end: 186.18, label: "ZSUI", accent: "#fb7185" },
  { start: 186.18, end: 216, label: "测试", accent: "#60a5fa" },
];

const baseText: React.CSSProperties = {
  fontFamily: "'Microsoft YaHei UI', 'Segoe UI', sans-serif",
  letterSpacing: 0,
};

const mediaFrameStyle: React.CSSProperties = {
  position: "absolute",
  overflow: "hidden",
  borderRadius: 18,
  boxShadow: "0 22px 70px rgba(0,0,0,0.38)",
  border: "1px solid rgba(255,255,255,0.16)",
  backgroundColor: "#0f172a",
};

const asset = (name: string) => staticFile(`ai-competition/${name}`);

const sceneEase = Easing.bezier(0.16, 1, 0.3, 1);

const entrance = (frame: number, duration: number) => {
  const enter = interpolate(frame, [0, 18], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });
  const exit = interpolate(frame, [duration - 18, duration], [1, 0], {
    ...clampOptions,
    easing: Easing.in(Easing.cubic),
  });
  return Math.min(enter, exit);
};

const Background = ({ accent }: { accent: string }) => {
  const frame = useCurrentFrame();
  const drift = interpolate(frame, [0, ZSCLIP_AI_TOTAL_FRAMES], [0, 180]);

  return (
    <AbsoluteFill
      style={{
        background:
          "linear-gradient(135deg, #101214 0%, #171a1f 42%, #0f1412 100%)",
      }}
    >
      <div
        style={{
          position: "absolute",
          inset: 0,
          backgroundImage:
            "linear-gradient(rgba(255,255,255,0.055) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.055) 1px, transparent 1px)",
          backgroundSize: "48px 48px",
          transform: `translate(${-drift % 48}px, ${-(drift * 0.5) % 48}px)`,
          opacity: 0.36,
        }}
      />
      <div
        style={{
          position: "absolute",
          left: -120,
          top: 48,
          width: 520,
          height: 2,
          background: `linear-gradient(90deg, transparent, ${accent}, transparent)`,
          opacity: 0.65,
        }}
      />
      <div
        style={{
          position: "absolute",
          right: -80,
          bottom: 112,
          width: 620,
          height: 2,
          background: `linear-gradient(90deg, transparent, ${accent}, transparent)`,
          opacity: 0.36,
        }}
      />
    </AbsoluteFill>
  );
};

const Kicker = ({
  chapter,
  localFrame,
}: {
  chapter: Chapter;
  localFrame: number;
}) => {
  const p = interpolate(localFrame, [0, 24], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });

  return (
    <div
      style={{
        ...baseText,
        position: "absolute",
        left: 46,
        top: 36,
        display: "flex",
        alignItems: "center",
        gap: 12,
        opacity: p,
        transform: `translateY(${(1 - p) * -14}px)`,
      }}
    >
      <div
        style={{
          width: 12,
          height: 12,
          borderRadius: 999,
          backgroundColor: chapter.accent,
          boxShadow: `0 0 22px ${chapter.accent}`,
        }}
      />
      <span
        style={{
          color: "rgba(255,255,255,0.72)",
          fontSize: 18,
          fontWeight: 700,
        }}
      >
        {chapter.label}
      </span>
      <span style={{ color: "rgba(255,255,255,0.34)", fontSize: 18 }}>/</span>
      <span
        style={{
          color: "white",
          fontSize: 21,
          fontWeight: 800,
        }}
      >
        {chapter.title}
      </span>
    </div>
  );
};

const Callout = ({
  x,
  y,
  text,
  accent,
  delay,
  frame,
  width = 270,
}: {
  x: number;
  y: number;
  text: string;
  accent: string;
  delay: number;
  frame: number;
  width?: number;
}) => {
  const p = interpolate(frame - delay, [0, 18], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });

  return (
    <div
      style={{
        ...baseText,
        position: "absolute",
        left: x,
        top: y,
        width,
        padding: "12px 16px",
        borderRadius: 16,
        color: "white",
        backgroundColor: "rgba(15, 18, 24, 0.78)",
        border: `1px solid ${accent}88`,
        boxShadow: `0 12px 42px rgba(0,0,0,0.30), 0 0 24px ${accent}33`,
        opacity: p,
        transform: `translateY(${(1 - p) * 18}px) scale(${0.96 + p * 0.04})`,
        fontSize: 19,
        fontWeight: 800,
        lineHeight: 1.35,
      }}
    >
      {text}
    </div>
  );
};

const SceneShell = ({
  chapter,
  localFrame,
  duration,
  children,
}: {
  chapter: Chapter;
  localFrame: number;
  duration: number;
  children: React.ReactNode;
}) => {
  const p = entrance(localFrame, duration);
  return (
    <AbsoluteFill style={{ opacity: p }}>
      <Background accent={chapter.accent} />
      {children}
      <Kicker chapter={chapter} localFrame={localFrame} />
    </AbsoluteFill>
  );
};

const GeneratedArt = ({
  src,
  frame,
  style,
  opacity = 0.62,
  scale = [1, 1.08],
}: {
  src: string;
  frame: number;
  style?: React.CSSProperties;
  opacity?: number;
  scale?: [number, number];
}) => {
  const zoom = interpolate(frame, [0, seconds(36)], scale, clampOptions);
  const drift = interpolate(frame, [0, seconds(36)], [-12, 16], clampOptions);

  return (
    <Img
      src={asset(src)}
      style={{
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        objectFit: "cover",
        opacity,
        transform: `translate3d(${drift}px, 0, 0) scale(${zoom})`,
        filter: "saturate(0.95) contrast(1.04)",
        ...style,
      }}
    />
  );
};

const IntroScene = ({ frame, duration }: { frame: number; duration: number }) => {
  const chapter = chapters[0];
  const title = interpolate(frame, [0, 34], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });
  const panel = interpolate(frame, [18, 48], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });
  const image = interpolate(frame, [36, 72], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <div
        style={{
          ...baseText,
          position: "absolute",
          left: 64,
          top: 126,
          width: 500,
          color: "white",
        }}
      >
        <div
          style={{
            fontSize: 92,
            lineHeight: 0.94,
            fontWeight: 900,
            opacity: title,
            transform: `translateX(${(1 - title) * -46}px)`,
          }}
        >
          ZSClip
        </div>
        <div
          style={{
            marginTop: 24,
            fontSize: 30,
            lineHeight: 1.28,
            color: "rgba(255,255,255,0.84)",
            opacity: title,
            transform: `translateY(${(1 - title) * 20}px)`,
          }}
        >
          一个从 Python 小工具一路迭代到 Rust 原生 UI 的本地办公剪贴板
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: 64,
          top: 392,
          display: "grid",
          gap: 10,
          opacity: panel,
          transform: `translateY(${(1 - panel) * 26}px)`,
        }}
      >
        {["复制记录", "VV 候选粘贴", "OCR 素材化", "组合搜索"].map((item, i) => (
          <div
            key={item}
            style={{
              ...baseText,
              width: 310,
              padding: "11px 18px",
              color: "white",
              fontSize: 22,
              fontWeight: 800,
              borderRadius: 16,
              backgroundColor: "rgba(255,255,255,0.075)",
              border: "1px solid rgba(255,255,255,0.13)",
              transform: `translateX(${Math.sin((frame + i * 16) / 18) * 5}px)`,
            }}
          >
            <span style={{ color: chapter.accent, marginRight: 12 }}>0{i + 1}</span>
            {item}
          </div>
        ))}
      </div>
      <div
        style={{
          ...mediaFrameStyle,
          right: 54,
          top: 128,
          width: 642,
          height: 424,
          opacity: image,
          transform: `translateX(${(1 - image) * 62}px) scale(${0.92 + image * 0.08})`,
        }}
      >
        <GeneratedArt
          src="scene-python-rust-fragments.png"
          frame={frame}
          opacity={0.78}
          scale={[1.02, 1.1]}
        />
        <Img
          src={asset("main-window.png")}
          style={{
            position: "absolute",
            right: 26,
            bottom: 26,
            width: 430,
            height: 246,
            objectFit: "cover",
            borderRadius: 14,
            boxShadow: "0 18px 42px rgba(0,0,0,0.42)",
            border: "1px solid rgba(255,255,255,0.2)",
          }}
        />
      </div>
      <Callout
        x={786}
        y={580}
        width={370}
        text="不是一个模板演示，而是每天复制资料时真会打开的工具"
        accent={chapter.accent}
        delay={78}
        frame={frame}
      />
    </SceneShell>
  );
};

const NativeUiScene = ({
  frame,
  duration,
}: {
  frame: number;
  duration: number;
}) => {
  const chapter = chapters[1];
  const p = interpolate(frame, [0, 28], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <GeneratedArt
        src="scene-zsui-open-engineering.png"
        frame={frame}
        opacity={0.28}
        scale={[1.01, 1.06]}
      />
      <div
        style={{
          ...mediaFrameStyle,
          left: 48,
          top: 92,
          width: 760,
          height: 500,
          transform: `translateY(${(1 - p) * 36}px) scale(${0.96 + p * 0.04})`,
        }}
      >
        <GeneratedArt
          src="scene-zsui-open-engineering.png"
          frame={frame}
          opacity={0.74}
          scale={[1.04, 1.12]}
        />
        <div
          style={{
            ...baseText,
            position: "absolute",
            left: 34,
            bottom: 30,
            width: 480,
            color: "white",
            fontSize: 28,
            lineHeight: 1.24,
            fontWeight: 900,
            textShadow: "0 3px 16px rgba(0,0,0,0.62)",
          }}
        >
          Rust native system tool UI
        </div>
      </div>
      <Sequence
        from={seconds(16)}
        durationInFrames={Math.max(1, Math.min(seconds(20), duration - seconds(16)))}
      >
        <div
          style={{
            ...mediaFrameStyle,
            right: 56,
            bottom: 92,
            width: 520,
            height: 292,
            transform: "rotate(0.6deg)",
          }}
        >
          <Video
            src={asset("desktop-demo.mp4")}
            muted
            objectFit="cover"
            style={{ width: "100%", height: "100%" }}
          />
        </div>
      </Sequence>
      <div
        style={{
          ...baseText,
          position: "absolute",
          right: 54,
          top: 116,
          width: 310,
          color: "white",
          opacity: p,
        }}
      >
        <div style={{ fontSize: 40, fontWeight: 900, lineHeight: 1.12 }}>
          常驻工具要贴近系统
        </div>
        <div
          style={{
            marginTop: 20,
            color: "rgba(255,255,255,0.72)",
            fontSize: 22,
            lineHeight: 1.45,
            fontWeight: 600,
          }}
        >
          热键、托盘、剪贴板、窗口，都尽量走原生路径。
        </div>
      </div>
      <Callout x={910} y={338} text="不是网页壳" accent={chapter.accent} delay={40} frame={frame} />
      <Callout x={910} y={416} text="低占用常驻" accent="#22c55e" delay={56} frame={frame} />
      <Callout x={910} y={494} text="纯原生 UI 方向" accent="#f97316" delay={72} frame={frame} />
    </SceneShell>
  );
};

const VvScene = ({ frame, duration }: { frame: number; duration: number }) => {
  const chapter = chapters[2];
  const zoom = interpolate(frame, [0, duration], [1.02, 1.08], clampOptions);
  const pulse = 0.5 + Math.sin(frame / 8) * 0.5;

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <div
        style={{
          ...mediaFrameStyle,
          left: 42,
          top: 84,
          width: 1196,
          height: 556,
        }}
      >
        <Video
          src={asset("desktop-demo.mp4")}
          muted
          objectFit="cover"
          style={{
            width: "100%",
            height: "100%",
            transform: `scale(${zoom})`,
          }}
        />
      </div>
      <Callout
        x={72}
        y={528}
        width={405}
        text="输入 vv，候选窗贴着光标出来，按数字就粘贴"
        accent={chapter.accent}
        delay={18}
        frame={frame}
      />
      <div
        style={{
          position: "absolute",
          left: 748,
          top: 284,
          width: 256,
          height: 128,
          borderRadius: 22,
          border: `4px solid rgba(249,115,22,${0.45 + pulse * 0.35})`,
          boxShadow: `0 0 46px rgba(249,115,22,${0.18 + pulse * 0.24})`,
        }}
      />
      <div
        style={{
          ...baseText,
          position: "absolute",
          right: 78,
          bottom: 104,
          color: "white",
          fontSize: 64,
          fontWeight: 900,
          opacity: interpolate(frame, [42, 68], [0, 1], {
            ...clampOptions,
            easing: sceneEase,
          }),
        }}
      >
        vv
      </div>
    </SceneShell>
  );
};

const SearchScene = ({
  frame,
  duration,
}: {
  frame: number;
  duration: number;
}) => {
  const chapter = chapters[3];
  const chips = ["应用", "日期", "时间", "类型", "右键分组", "附近记录"];

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <div
        style={{
          ...mediaFrameStyle,
          left: 46,
          top: 88,
          width: 870,
          height: 520,
        }}
      >
        <Video
          src={asset("feature-closeups.mp4")}
          muted
          objectFit="cover"
          style={{ width: "100%", height: "100%" }}
        />
      </div>
      <div
        style={{
          ...baseText,
          position: "absolute",
          right: 42,
          top: 112,
          width: 330,
          color: "white",
          fontSize: 34,
          lineHeight: 1.15,
          fontWeight: 900,
        }}
      >
        搜索不是只搜文字，分组也不用绕路
      </div>
      <div
        style={{
          position: "absolute",
          right: 52,
          top: 262,
          display: "grid",
          gap: 11,
        }}
      >
        {chips.map((chip, i) => {
          const p = interpolate(frame - i * 10, [18, 38], [0, 1], {
            ...clampOptions,
            easing: sceneEase,
          });
          return (
            <div
              key={chip}
              style={{
                ...baseText,
                width: 252,
                padding: "12px 16px",
                borderRadius: 16,
                color: "white",
                backgroundColor: "rgba(255,255,255,0.08)",
                border: `1px solid ${chapter.accent}77`,
                fontSize: 22,
                fontWeight: 800,
                opacity: p,
                transform: `translateX(${(1 - p) * 36}px)`,
              }}
            >
              {chip}
            </div>
          );
        })}
      </div>
    </SceneShell>
  );
};

const OcrScene = ({ frame, duration }: { frame: number; duration: number }) => {
  const chapter = chapters[4];
  const steps = ["截图", "OCR", "清洗", "翻译", "短语"];

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <div
        style={{
          ...mediaFrameStyle,
          right: 48,
          top: 92,
          width: 800,
          height: 500,
        }}
      >
        <Video
          src={asset("feature-closeups.mp4")}
          muted
          trimBefore={seconds(23)}
          objectFit="cover"
          style={{ width: "100%", height: "100%" }}
        />
      </div>
      <div
        style={{
          ...baseText,
          position: "absolute",
          left: 70,
          top: 148,
          width: 310,
          color: "white",
        }}
      >
        <div style={{ fontSize: 43, lineHeight: 1.12, fontWeight: 900 }}>
          复制过的东西重新变成素材
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: 70,
          top: 372,
          display: "flex",
          gap: 10,
          flexWrap: "wrap",
          width: 420,
        }}
      >
        {steps.map((step, i) => {
          const p = interpolate(frame - i * 12, [8, 26], [0, 1], {
            ...clampOptions,
            easing: sceneEase,
          });
          return (
            <div
              key={step}
              style={{
                ...baseText,
                padding: "12px 15px",
                borderRadius: 15,
                color: "white",
                backgroundColor: i === 1 ? chapter.accent : "rgba(255,255,255,0.085)",
                fontSize: 22,
                fontWeight: 900,
                opacity: p,
                transform: `translateY(${(1 - p) * 22}px)`,
              }}
            >
              {step}
            </div>
          );
        })}
      </div>
    </SceneShell>
  );
};

const MemoryScene = ({
  frame,
  duration,
}: {
  frame: number;
  duration: number;
}) => {
  const chapter = chapters[5];
  const p = interpolate(frame, [0, 24], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });
  const sweep = interpolate(frame % 80, [0, 80], [-140, 860]);
  const rows = ["分页加载", "只画可见区域", "正文按需补全", "图片按需补全", "隐藏后释放缓存"];

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <GeneratedArt
        src="scene-memory-minimal.png"
        frame={frame}
        opacity={0.34}
        scale={[1.02, 1.08]}
      />
      <div
        style={{
          ...mediaFrameStyle,
          left: 46,
          top: 96,
          width: 820,
          height: 462,
          opacity: p,
        }}
      >
        <Img
          src={asset("task-manager-memory.png")}
          style={{ width: "100%", height: "100%", objectFit: "cover" }}
        />
        <div
          style={{
            position: "absolute",
            left: sweep,
            top: 0,
            width: 80,
            height: "100%",
            background:
              "linear-gradient(90deg, transparent, rgba(250,204,21,0.22), transparent)",
          }}
        />
      </div>
      <div
        style={{
          ...baseText,
          position: "absolute",
          right: 56,
          top: 120,
          width: 300,
          color: "white",
        }}
      >
        <div style={{ fontSize: 30, color: "rgba(255,255,255,0.72)", fontWeight: 800 }}>
          常驻工具的审美
        </div>
        <div
          style={{
            marginTop: 8,
            fontSize: 78,
            lineHeight: 1,
            fontWeight: 950,
            color: chapter.accent,
          }}
        >
          0.1M
        </div>
        <div style={{ marginTop: 8, fontSize: 28, fontWeight: 900 }}>
          级约束打磨
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          right: 56,
          top: 342,
          display: "grid",
          gap: 9,
        }}
      >
        {rows.map((row, i) => {
          const itemP = interpolate(frame - i * 8, [26, 44], [0, 1], {
            ...clampOptions,
            easing: sceneEase,
          });
          return (
            <div
              key={row}
              style={{
                ...baseText,
                width: 310,
                display: "flex",
                alignItems: "center",
                gap: 10,
                color: "rgba(255,255,255,0.86)",
                fontSize: 20,
                fontWeight: 800,
                opacity: itemP,
                transform: `translateX(${(1 - itemP) * 24}px)`,
              }}
            >
              <span
                style={{
                  display: "inline-block",
                  width: 9,
                  height: 9,
                  borderRadius: 999,
                  backgroundColor: chapter.accent,
                }}
              />
              {row}
            </div>
          );
        })}
      </div>
    </SceneShell>
  );
};

const ZsuiScene = ({ frame, duration }: { frame: number; duration: number }) => {
  const chapter = chapters[6];
  const cards = [
    ["合同", "托盘 / 热键 / 弹窗 / 列表"],
    ["贡献", "组件 / 平台 / 测试"],
    ["Rust", "类型安全 / 显式 / 低魔法"],
  ];

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <GeneratedArt
        src="scene-zsui-open-engineering.png"
        frame={frame}
        opacity={0.3}
        scale={[1.01, 1.06]}
      />
      <div
        style={{
          ...mediaFrameStyle,
          left: 44,
          top: 90,
          width: 760,
          height: 500,
        }}
      >
        <Video
          src={asset("dev-scroll.mp4")}
          muted
          objectFit="cover"
          style={{ width: "100%", height: "100%" }}
        />
      </div>
      <div
        style={{
          ...mediaFrameStyle,
          right: 60,
          top: 96,
          width: 344,
          height: 212,
          transform: `rotate(${interpolate(frame, [0, duration], [-1.4, 1.1])}deg)`,
        }}
      >
        <Img
          src={asset("token-usage.png")}
          style={{ width: "100%", height: "100%", objectFit: "cover" }}
        />
      </div>
      <div
        style={{
          position: "absolute",
          right: 60,
          top: 338,
          display: "grid",
          gap: 12,
        }}
      >
        {cards.map(([title, body], i) => {
          const p = interpolate(frame - i * 14, [18, 40], [0, 1], {
            ...clampOptions,
            easing: sceneEase,
          });
          return (
            <div
              key={title}
              style={{
                ...baseText,
                width: 386,
                padding: "15px 18px",
                borderRadius: 18,
                backgroundColor: "rgba(255,255,255,0.08)",
                border: `1px solid ${chapter.accent}77`,
                color: "white",
                opacity: p,
                transform: `translateX(${(1 - p) * 40}px)`,
              }}
            >
              <div style={{ color: chapter.accent, fontSize: 20, fontWeight: 900 }}>
                {title}
              </div>
              <div style={{ marginTop: 5, fontSize: 23, fontWeight: 850 }}>
                {body}
              </div>
            </div>
          );
        })}
      </div>
      <Callout
        x={94}
        y={548}
        width={610}
        text="ZSUI 不是另一个通用 UI 框架，而是 Rust 原生系统工具应用框架"
        accent={chapter.accent}
        delay={86}
        frame={frame}
      />
    </SceneShell>
  );
};

const EndingScene = ({
  frame,
  duration,
}: {
  frame: number;
  duration: number;
}) => {
  const chapter = chapters[7];
  const p = interpolate(frame, [0, 18], [0, 1], {
    ...clampOptions,
    easing: sceneEase,
  });
  const items = ["搜索：附近", "分组：文件类型", "继续打磨多平台"];

  return (
    <SceneShell chapter={chapter} localFrame={frame} duration={duration}>
      <GeneratedArt
        src="scene-zsui-open-engineering.png"
        frame={frame}
        opacity={0.36}
        scale={[1.04, 1.08]}
      />
      <div
        style={{
          ...baseText,
          position: "absolute",
          left: 78,
          top: 142,
          color: "white",
          opacity: p,
          transform: `translateY(${(1 - p) * 34}px)`,
        }}
      >
        <div style={{ fontSize: 62, fontWeight: 950 }}>下一步继续打磨</div>
        <div
          style={{
            marginTop: 18,
            color: "rgba(255,255,255,0.72)",
            fontSize: 28,
            fontWeight: 700,
          }}
        >
          macOS / Linux 还是初代版本，欢迎下载测试、加群反馈。
        </div>
      </div>
      <div style={{ position: "absolute", left: 82, top: 338, display: "flex", gap: 16 }}>
        {items.map((item, i) => {
          const itemP = interpolate(frame - i * 7, [8, 24], [0, 1], {
            ...clampOptions,
            easing: sceneEase,
          });
          return (
            <div
              key={item}
              style={{
                ...baseText,
                width: 310,
                padding: "24px 22px",
                borderRadius: 20,
                color: "white",
                backgroundColor: "rgba(255,255,255,0.09)",
                border: `1px solid ${chapter.accent}88`,
                fontSize: 26,
                fontWeight: 900,
                opacity: itemP,
                transform: `translateY(${(1 - itemP) * 24}px)`,
              }}
            >
              {item}
            </div>
          );
        })}
      </div>
    </SceneShell>
  );
};

const sceneComponents = [
  IntroScene,
  NativeUiScene,
  VvScene,
  SearchScene,
  OcrScene,
  MemoryScene,
  ZsuiScene,
  EndingScene,
];

const SceneLayer = () => {
  const frame = useCurrentFrame();

  return (
    <>
      {chapters.map((chapter, i) => {
        const Scene = sceneComponents[i];
        const from = seconds(chapter.start);
        const duration = seconds(chapter.end - chapter.start);
        return (
          <Sequence key={chapter.label} from={from} durationInFrames={duration}>
            <Scene frame={frame - from} duration={duration} />
          </Sequence>
        );
      })}
    </>
  );
};

const ChapterProgress = () => {
  const frame = useCurrentFrame();
  const progress = frame / (ZSCLIP_AI_TOTAL_FRAMES - 1);
  const active =
    progressChapters.find((chapter) => frame >= seconds(chapter.start) && frame < seconds(chapter.end)) ??
    progressChapters[progressChapters.length - 1];
  const { width } = useVideoConfig();

  return (
    <div
      style={{
        ...baseText,
        position: "absolute",
        left: 0,
        right: 0,
        bottom: 0,
        height: 74,
        background:
          "linear-gradient(180deg, rgba(15,16,18,0), rgba(15,16,18,0.74) 30%, rgba(15,16,18,0.96))",
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 44,
          right: 44,
          top: 28,
          height: 4,
          borderRadius: 999,
          backgroundColor: "rgba(255,255,255,0.17)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            width: `${progress * 100}%`,
            height: "100%",
            borderRadius: 999,
            background: `linear-gradient(90deg, #22c55e, ${active.accent})`,
          }}
        />
      </div>
      {progressChapters.map((chapter) => {
        const left = 44 + ((width - 88) * chapter.start) / TOTAL_SECONDS;
        const isActive = chapter.label === active.label;
        return (
          <div
            key={chapter.label}
            style={{
              position: "absolute",
              left,
              top: isActive ? 10 : 18,
              transform: "translateX(-50%)",
              display: "grid",
              justifyItems: "center",
              gap: 7,
            }}
          >
            <div
              style={{
                width: isActive ? 13 : 8,
                height: isActive ? 13 : 8,
                borderRadius: 999,
                backgroundColor: isActive ? chapter.accent : "rgba(255,255,255,0.55)",
                boxShadow: isActive ? `0 0 20px ${chapter.accent}` : "none",
              }}
            />
            <div
              style={{
                padding: isActive ? "5px 10px" : "0",
                borderRadius: 999,
                backgroundColor: isActive ? "rgba(255,255,255,0.10)" : "transparent",
                color: isActive ? "white" : "rgba(255,255,255,0.54)",
                fontSize: isActive ? 16 : 14,
                fontWeight: 850,
                whiteSpace: "nowrap",
              }}
            >
              {chapter.label}
            </div>
          </div>
        );
      })}
      <div
        style={{
          position: "absolute",
          right: 46,
          bottom: 13,
          color: "rgba(255,255,255,0.64)",
          fontSize: 14,
          fontWeight: 800,
        }}
      >
        {Math.floor(frame / FPS)
          .toString()
          .padStart(2, "0")}
        s / {TOTAL_SECONDS}s
      </div>
    </div>
  );
};

const TransitionOverlay = () => {
  const frame = useCurrentFrame();
  const boundary = chapters
    .slice(1)
    .map((chapter) => seconds(chapter.start))
    .find((point) => Math.abs(frame - point) <= 16);

  if (boundary === undefined) {
    return null;
  }

  const distance = Math.abs(frame - boundary);
  const strength = interpolate(distance, [0, 16], [1, 0], clampOptions);
  const next = chapters.find((chapter) => seconds(chapter.start) === boundary);
  const accent = next?.accent ?? "#ffffff";

  return (
    <AbsoluteFill
      style={{
        pointerEvents: "none",
        opacity: strength,
      }}
    >
      <div
        style={{
          position: "absolute",
          left: interpolate(strength, [0, 1], [-900, -120]),
          top: 0,
          width: 820,
          height: 720,
          transform: "skewX(-18deg)",
          background: `linear-gradient(90deg, transparent, ${accent}88, rgba(255,255,255,0.82), ${accent}66, transparent)`,
          filter: "blur(0.5px)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 0,
          height: "100%",
          backgroundColor: "rgba(255,255,255,0.08)",
        }}
      />
    </AbsoluteFill>
  );
};

const CoverIntroOverlay = () => {
  const frame = useCurrentFrame();
  const coverFrames = seconds(COVER_INTRO_SECONDS);

  if (frame > coverFrames + 12) {
    return null;
  }

  const opacity = interpolate(frame, [coverFrames - 12, coverFrames], [1, 0], {
    ...clampOptions,
    easing: sceneEase,
  });
  const scale = interpolate(frame, [0, coverFrames], [1.035, 1], clampOptions);
  const sweep = interpolate(frame, [coverFrames - 18, coverFrames], [-1280, 1280], {
    ...clampOptions,
    easing: Easing.inOut(Easing.cubic),
  });

  return (
    <AbsoluteFill
      style={{
        pointerEvents: "none",
        opacity,
        backgroundColor: "#08090b",
      }}
    >
      <Img
        src={asset("bilibili-cover-zsclip.png")}
        style={{
          width: "100%",
          height: "100%",
          objectFit: "cover",
          transform: `scale(${scale})`,
          filter: "saturate(1.03) contrast(1.02)",
        }}
      />
      <div
        style={{
          position: "absolute",
          inset: 0,
          background:
            "linear-gradient(90deg, rgba(8,9,11,0.20), rgba(8,9,11,0.00) 42%, rgba(8,9,11,0.24))",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: sweep,
          top: 0,
          width: 280,
          height: "100%",
          transform: "skewX(-16deg)",
          background:
            "linear-gradient(90deg, transparent, rgba(255,255,255,0.74), transparent)",
          opacity: interpolate(frame, [coverFrames - 18, coverFrames - 4], [0, 0.75], clampOptions),
          filter: "blur(1px)",
        }}
      />
    </AbsoluteFill>
  );
};

export const ZsclipAiCompetition = () => {
  return (
    <AbsoluteFill>
      <SceneLayer />
      <TransitionOverlay />
      <ChapterProgress />
      <CoverIntroOverlay />
    </AbsoluteFill>
  );
};

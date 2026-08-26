import {Audio} from "@remotion/media";
import {
  AbsoluteFill,
  Easing,
  Img,
  Sequence,
  interpolate,
  staticFile,
  useCurrentFrame,
} from "remotion";
import type {CSSProperties, ReactNode} from "react";

const accent = "#0078d4";
const green = "#20a66a";
const amber = "#f2a51a";
const ink = "#102033";
const muted = "#607286";
const panel = "rgba(255,255,255,0.9)";
const shadow = "0 28px 76px rgba(30, 64, 98, 0.18)";

const sceneDurations = {
  hero: 150,
  discover: 240,
  pairing: 270,
  modes: 270,
  formats: 270,
  android: 300,
  safety: 180,
  cta: 120,
};

const icon = (name: string) => staticFile(`icons/${name}.png`);
const audio = (name: string) => staticFile(`audio/${name}`);

const soft = (frame: number, start: number, end: number, from: number, to: number) =>
  interpolate(frame, [start, end], [from, to], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.16, 1, 0.3, 1),
  });

const fade = (frame: number, duration: number) => {
  const fadeIn = interpolate(frame, [0, 22], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const fadeOut = interpolate(frame, [duration - 22, duration], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return Math.min(fadeIn, fadeOut);
};

const card: CSSProperties = {
  borderRadius: 30,
  border: "1px solid rgba(120, 146, 174, 0.24)",
  background: panel,
  boxShadow: shadow,
};

const SceneShell = ({
  children,
  duration,
  subtitle,
}: {
  children: ReactNode;
  duration: number;
  subtitle: string;
}) => {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill
      style={{
        opacity: fade(frame, duration),
        color: ink,
        fontFamily:
          '"Microsoft YaHei UI", "Segoe UI", "PingFang SC", sans-serif',
      }}
    >
      <Backdrop />
      {children}
      <Subtitle text={subtitle} />
    </AbsoluteFill>
  );
};

const Backdrop = () => {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill
      style={{
        overflow: "hidden",
        background:
          "radial-gradient(circle at 17% 12%, #dff1ff 0, transparent 32%), radial-gradient(circle at 88% 14%, #dcf4e9 0, transparent 30%), linear-gradient(135deg, #f8fbff 0%, #eff5fb 54%, #f9fcf8 100%)",
      }}
    >
      <div
        style={{
          position: "absolute",
          inset: 0,
          opacity: 0.55,
          backgroundImage:
            "linear-gradient(rgba(28, 74, 112, 0.06) 1px, transparent 1px), linear-gradient(90deg, rgba(28, 74, 112, 0.06) 1px, transparent 1px)",
          backgroundSize: "56px 56px",
          transform: `translateY(${-(frame % 56)}px)`,
        }}
      />
      <div
        style={{
          position: "absolute",
          left: -230 + Math.sin(frame / 72) * 18,
          bottom: -260,
          width: 760,
          height: 760,
          borderRadius: "50%",
          background: "rgba(0, 120, 212, 0.12)",
          filter: "blur(36px)",
        }}
      />
      <div
        style={{
          position: "absolute",
          right: -150,
          top: -120 + Math.cos(frame / 80) * 16,
          width: 560,
          height: 560,
          borderRadius: "50%",
          background: "rgba(32, 166, 106, 0.14)",
          filter: "blur(32px)",
        }}
      />
    </AbsoluteFill>
  );
};

const Subtitle = ({text}: {text: string}) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: "absolute",
        left: 205,
        right: 205,
        bottom: 54,
        height: 82,
        borderRadius: 32,
        background: "rgba(16, 32, 51, 0.82)",
        boxShadow: "0 18px 48px rgba(16, 32, 51, 0.22)",
        color: "white",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: 33,
        fontWeight: 850,
        letterSpacing: 0.5,
        transform: `translateY(${soft(frame, 0, 22, 26, 0)}px)`,
      }}
    >
      {text}
    </div>
  );
};

const LogoMark = ({size = 96}: {size?: number}) => (
  <div
    style={{
      width: size,
      height: size,
      borderRadius: size * 0.25,
      border: "1px solid rgba(0,120,212,0.2)",
      background: "linear-gradient(145deg, #ffffff, #e7f5ff)",
      boxShadow: "0 18px 42px rgba(0,120,212,0.18)",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
    }}
  >
    <Img src={icon("text")} style={{width: size * 0.62, height: size * 0.62}} />
  </div>
);

const TitleBlock = ({
  eyebrow,
  title,
  detail,
  width = 760,
}: {
  eyebrow: string;
  title: string;
  detail: string;
  width?: number;
}) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: "absolute",
        left: 118,
        top: 100,
        width,
        transform: `translateY(${soft(frame, 0, 28, 36, 0)}px)`,
      }}
    >
      <div style={{fontSize: 25, color: accent, fontWeight: 900}}>{eyebrow}</div>
      <div
        style={{
          fontSize: 62,
          fontWeight: 950,
          letterSpacing: -1.5,
          marginTop: 18,
          lineHeight: 1.12,
        }}
      >
        {title}
      </div>
      <div style={{fontSize: 25, color: muted, lineHeight: 1.55, marginTop: 22}}>
        {detail}
      </div>
    </div>
  );
};

const MockWindow = ({
  title,
  children,
  style,
}: {
  title: string;
  children: ReactNode;
  style?: CSSProperties;
}) => (
  <div
    style={{
      ...card,
      overflow: "hidden",
      background: "rgba(255,255,255,0.94)",
      ...style,
    }}
  >
    <div
      style={{
        height: 74,
        borderBottom: "1px solid #e5edf5",
        display: "flex",
        alignItems: "center",
        padding: "0 24px",
        gap: 16,
      }}
    >
      <LogoMark size={44} />
      <div style={{fontSize: 23, fontWeight: 900}}>{title}</div>
      <div style={{marginLeft: "auto", display: "flex", gap: 16, color: "#1f2d3d"}}>
        <span>⌕</span>
        <span>⚙</span>
        <span>−</span>
        <span>×</span>
      </div>
    </div>
    {children}
  </div>
);

const DeviceCard = ({
  label,
  detail,
  type,
  x,
  y,
  delay,
  active,
}: {
  label: string;
  detail: string;
  type: "pc" | "phone";
  x: number;
  y: number;
  delay: number;
  active?: boolean;
}) => {
  const frame = useCurrentFrame();
  const instant = delay <= 0;
  const width = type === "pc" ? 360 : 220;
  const height = type === "pc" ? 235 : 330;
  return (
    <div
      style={{
        ...card,
        position: "absolute",
        left: x,
        top: y,
        width,
        height,
        padding: 26,
        textAlign: "center",
        opacity: instant ? 1 : soft(frame, delay, delay + 22, 0, 1),
        transform: `translateY(${instant ? 0 : soft(frame, delay, delay + 26, 42, 0)}px)`,
        outline: active ? `4px solid ${accent}` : "none",
      }}
    >
      <div
        style={{
          margin: "0 auto 18px",
          width: type === "pc" ? 250 : 102,
          height: type === "pc" ? 134 : 184,
          borderRadius: type === "pc" ? 18 : 30,
          border: "10px solid #26384c",
          background:
            type === "pc"
              ? "linear-gradient(135deg, #f8fbff, #d7ecff)"
              : "linear-gradient(135deg, #f9fff9, #d8f5e6)",
          boxShadow: "inset 0 0 0 1px rgba(255,255,255,0.7)",
        }}
      />
      <div style={{fontSize: 25, fontWeight: 950}}>{label}</div>
      <div style={{fontSize: 18, color: muted, marginTop: 8}}>{detail}</div>
    </div>
  );
};

const Packet = ({
  fromX,
  fromY,
  toX,
  toY,
  color,
  delay = 0,
}: {
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  color: string;
  delay?: number;
}) => {
  const frame = useCurrentFrame();
  const local = Math.max(0, frame - delay);
  const t = (local % 86) / 86;
  const x = fromX + (toX - fromX) * t;
  const y = fromY + (toY - fromY) * t - Math.sin(t * Math.PI) * 88;
  return (
    <div
      style={{
        position: "absolute",
        left: x - 17,
        top: y - 17,
        width: 34,
        height: 34,
        borderRadius: "50%",
        background: color,
        boxShadow: `0 0 0 12px ${color}22, 0 12px 30px ${color}55`,
      }}
    />
  );
};

const Pill = ({children, color = accent}: {children: ReactNode; color?: string}) => (
  <div
    style={{
      borderRadius: 999,
      padding: "12px 22px",
      fontSize: 20,
      fontWeight: 900,
      color,
      background: `${color}16`,
      border: `1px solid ${color}34`,
      whiteSpace: "nowrap",
    }}
  >
    {children}
  </div>
);

const AudioTracks = () => (
  <>
    <Audio
      src={audio("bgm.wav")}
      volume={(frame) =>
        interpolate(frame, [0, 90, 1710, 1800], [0, 0.22, 0.22, 0], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        })
      }
    />
    <Sequence durationInFrames={sceneDurations.hero}>
      <Audio src={audio("voice-01.wav")} volume={1} />
    </Sequence>
    <Sequence from={150} durationInFrames={sceneDurations.discover}>
      <Audio src={audio("voice-02.wav")} volume={1} />
    </Sequence>
    <Sequence from={390} durationInFrames={sceneDurations.pairing}>
      <Audio src={audio("voice-03.wav")} volume={1} />
    </Sequence>
    <Sequence from={660} durationInFrames={sceneDurations.modes}>
      <Audio src={audio("voice-04.wav")} volume={1} />
    </Sequence>
    <Sequence from={930} durationInFrames={sceneDurations.formats}>
      <Audio src={audio("voice-05.wav")} volume={1} />
    </Sequence>
    <Sequence from={1200} durationInFrames={sceneDurations.android}>
      <Audio src={audio("voice-06.wav")} volume={1} />
    </Sequence>
    <Sequence from={1500} durationInFrames={sceneDurations.safety}>
      <Audio src={audio("voice-07.wav")} volume={1} />
    </Sequence>
    <Sequence from={1680} durationInFrames={sceneDurations.cta}>
      <Audio src={audio("voice-08.wav")} volume={1} />
    </Sequence>
  </>
);

const HeroScene = () => {
  const frame = useCurrentFrame();
  return (
    <SceneShell duration={sceneDurations.hero} subtitle="0.9.0 多端同步预览：让剪贴板在设备之间自然流动">
      <div style={{position: "absolute", left: 145, top: 150}}>
        <LogoMark size={128} />
        <div style={{fontSize: 84, fontWeight: 950, marginTop: 42, letterSpacing: -2}}>
          ZSClip 0.9.0
        </div>
        <div style={{fontSize: 50, fontWeight: 930, marginTop: 16}}>多端同步预览版</div>
        <div style={{fontSize: 28, color: muted, marginTop: 26}}>
          Windows 多设备 · 安卓快捷开关 · 局域网本地传输
        </div>
      </div>
      <DeviceCard label="办公电脑" detail="复制记录" type="pc" x={1080} y={150} delay={8} active />
      <DeviceCard label="家用电脑" detail="自动发现" type="pc" x={1360} y={510} delay={24} />
      <DeviceCard label="Android" detail="快捷拉取" type="phone" x={850} y={485} delay={38} />
      <Packet fromX={1220} fromY={340} toX={1460} toY={610} color={accent} delay={22} />
      <Packet fromX={1160} fromY={340} toX={950} toY={610} color={green} delay={45} />
      <div
        style={{
          ...card,
          position: "absolute",
          left: 145,
          bottom: 190,
          padding: "20px 28px",
          display: "flex",
          gap: 18,
          opacity: soft(frame, 42, 72, 0, 1),
        }}
      >
        <Pill>不走云端</Pill>
        <Pill color={green}>配对信任</Pill>
        <Pill color={amber}>文本 / 图片 / 小文件</Pill>
      </div>
    </SceneShell>
  );
};

const DiscoverScene = () => (
  <SceneShell duration={sceneDurations.discover} subtitle="开启局域网同步，同网段设备自动发现">
    <TitleBlock
      eyebrow="Step 1"
      title="打开开关，先发现设备"
      detail="不需要每台设备反复输入 IP。ZSClip 通过 UDP 发现同网段设备，再用 TCP API 传输真实内容。"
    />
    <MockWindow title="设置 · 局域网" style={{position: "absolute", right: 118, top: 100, width: 810, height: 720}}>
      <div style={{padding: 28}}>
        <div style={{display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 22}}>
          <div>
            <div style={{fontSize: 26, fontWeight: 930}}>本机状态</div>
            <div style={{fontSize: 20, color: muted, marginTop: 8}}>剪贴板-PC · 192.168.1.18:38473</div>
          </div>
          <div style={{borderRadius: 999, background: `${green}18`, color: green, padding: "12px 22px", fontSize: 20, fontWeight: 900}}>
            已开启
          </div>
        </div>
        <div style={{display: "flex", gap: 14, marginBottom: 22}}>
          <div style={{borderRadius: 16, background: accent, color: "white", padding: "16px 26px", fontSize: 21, fontWeight: 900}}>
            刷新发现
          </div>
          <div style={{borderRadius: 16, background: "#eef4fb", padding: "16px 26px", fontSize: 21, fontWeight: 900}}>
            允许配对
          </div>
        </div>
        {[
          ["家用电脑", "192.168.1.25", "可配对"],
          ["会议室电脑", "192.168.1.41", "已信任"],
          ["Android 手机", "192.168.1.63", "待配对"],
        ].map((row, index) => (
          <div
            key={row[0]}
            style={{
              height: 84,
              borderRadius: 18,
              background: index === 0 ? "rgba(0,120,212,0.12)" : "#f7f9fc",
              marginBottom: 16,
              display: "flex",
              alignItems: "center",
              padding: "0 22px",
              gap: 16,
              fontSize: 22,
              fontWeight: 850,
            }}
          >
            <div style={{width: 14, height: 14, borderRadius: "50%", background: index === 1 ? green : accent}} />
            <div style={{width: 190}}>{row[0]}</div>
            <div style={{color: muted, width: 170, fontWeight: 650}}>{row[1]}</div>
            <div style={{marginLeft: "auto", color: index === 1 ? green : accent}}>{row[2]}</div>
          </div>
        ))}
      </div>
    </MockWindow>
    <DeviceCard label="办公电脑" detail="广播发现包" type="pc" x={140} y={470} delay={28} active />
    <DeviceCard label="家用电脑" detail="响应在线状态" type="pc" x={540} y={470} delay={48} />
    <Packet fromX={490} fromY={590} toX={540} toY={590} color={accent} delay={42} />
  </SceneShell>
);

const PairingScene = () => (
  <SceneShell duration={sceneDurations.pairing} subtitle="配对不再弹来弹去：设置页里点设备，对方点允许">
    <TitleBlock
      eyebrow="Step 2"
      title="配对交互全部放进设置页"
      detail="点击附近设备发起配对，对方设置页同一列表里出现请求。点允许后，双方保存信任 token。"
      width={700}
    />
    <div style={{position: "absolute", right: 105, top: 118, display: "flex", gap: 28}}>
      <MockWindow title="A 电脑 · 发起配对" style={{width: 620, height: 640}}>
        <div style={{padding: 24}}>
          <div style={{fontSize: 25, fontWeight: 930, marginBottom: 18}}>附近设备</div>
          <div style={{height: 92, borderRadius: 20, background: "rgba(0,120,212,0.13)", display: "flex", alignItems: "center", padding: "0 22px", gap: 16}}>
            <DeviceDot color={accent} />
            <div>
              <div style={{fontSize: 24, fontWeight: 900}}>家用电脑</div>
              <div style={{fontSize: 18, color: muted}}>192.168.1.25 · 可配对</div>
            </div>
            <div style={{marginLeft: "auto", borderRadius: 15, background: accent, color: "white", padding: "14px 22px", fontSize: 19, fontWeight: 900}}>
              配对
            </div>
          </div>
          <div style={{fontSize: 20, color: muted, lineHeight: 1.55, marginTop: 34}}>
            不需要输入确认码，也不需要切到弹窗。
            <br />
            请求状态会在设置页内刷新。
          </div>
        </div>
      </MockWindow>
      <MockWindow title="B 电脑 · 收到请求" style={{width: 620, height: 640}}>
        <div style={{padding: 24}}>
          <div style={{fontSize: 25, fontWeight: 930, marginBottom: 18}}>待确认请求</div>
          <div style={{height: 112, borderRadius: 20, background: "#fff7e8", border: "1px solid #f4d69a", display: "flex", alignItems: "center", padding: "0 22px", gap: 16}}>
            <DeviceDot color={amber} />
            <div>
              <div style={{fontSize: 24, fontWeight: 900}}>办公电脑请求配对</div>
              <div style={{fontSize: 18, color: muted}}>安全码 524927 · 刚刚</div>
            </div>
          </div>
          <div style={{display: "flex", gap: 16, marginTop: 26}}>
            <div style={{borderRadius: 16, background: green, color: "white", padding: "16px 44px", fontSize: 21, fontWeight: 900}}>
              允许
            </div>
            <div style={{borderRadius: 16, background: "#eef2f6", padding: "16px 44px", fontSize: 21, fontWeight: 900}}>
              拒绝
            </div>
          </div>
          <div style={{fontSize: 20, color: muted, marginTop: 30}}>允许后进入已信任设备，后续自动同步。</div>
        </div>
      </MockWindow>
    </div>
  </SceneShell>
);

const DeviceDot = ({color}: {color: string}) => (
  <div
    style={{
      width: 44,
      height: 44,
      borderRadius: 14,
      background: `${color}18`,
      border: `2px solid ${color}55`,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
    }}
  >
    <div style={{width: 14, height: 14, borderRadius: "50%", background: color}} />
  </div>
);

const ModesScene = () => (
  <SceneShell duration={sceneDurations.modes} subtitle="同步方式可选：只进记录，或直接覆盖系统剪贴板">
    <TitleBlock
      eyebrow="Step 3"
      title="同步不是只有一种答案"
      detail="工作电脑更适合只进入记录；个人设备可以选择直接覆盖剪贴板。模式清楚，才不会误粘贴。"
      width={760}
    />
    <div style={{position: "absolute", right: 135, top: 190, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 28}}>
      <ModeCard title="只进入记录" badge="默认" color={accent} detail="远端内容只出现在 ZSClip 列表顶部，不改系统剪贴板。" />
      <ModeCard title="覆盖剪贴板" badge="可选" color={green} detail="入库后同步写入系统剪贴板，并打忽略标记防循环。" />
    </div>
    <MockWindow title="接收端记录" style={{position: "absolute", left: 155, bottom: 175, width: 720, height: 330}}>
      <div style={{padding: 24}}>
        <SyncRow iconName="text" title="LAN: 办公电脑" detail="客户回复模板 · 已进入记录" active />
        <SyncRow iconName="image" title="LAN: 家用电脑" detail="截图 1920×1080 · 未覆盖剪贴板" />
      </div>
    </MockWindow>
  </SceneShell>
);

const ModeCard = ({
  title,
  badge,
  detail,
  color,
}: {
  title: string;
  badge: string;
  detail: string;
  color: string;
}) => (
  <div style={{...card, width: 360, height: 260, padding: 30}}>
    <div style={{display: "flex", alignItems: "center", justifyContent: "space-between"}}>
      <div style={{fontSize: 31, fontWeight: 950}}>{title}</div>
      <Pill color={color}>{badge}</Pill>
    </div>
    <div style={{fontSize: 21, color: muted, lineHeight: 1.55, marginTop: 30}}>{detail}</div>
  </div>
);

const SyncRow = ({
  iconName,
  title,
  detail,
  active,
}: {
  iconName: string;
  title: string;
  detail: string;
  active?: boolean;
}) => (
  <div
    style={{
      height: 86,
      borderRadius: 18,
      background: active ? "rgba(0,120,212,0.12)" : "#f7f9fc",
      display: "flex",
      alignItems: "center",
      gap: 16,
      padding: "0 18px",
      marginBottom: 14,
    }}
  >
    <div style={{width: 48, height: 48, borderRadius: 12, border: "2px solid #263443", display: "flex", alignItems: "center", justifyContent: "center", background: "white"}}>
      <Img src={icon(iconName)} style={{width: 30, height: 30}} />
    </div>
    <div>
      <div style={{fontSize: 23, fontWeight: 900}}>{title}</div>
      <div style={{fontSize: 17, color: muted, marginTop: 5}}>{detail}</div>
    </div>
  </div>
);

const FormatsScene = () => (
  <SceneShell duration={sceneDurations.formats} subtitle="文本、图片、小文件自动同步；大文件不偷偷拖慢电脑">
    <TitleBlock
      eyebrow="Step 4"
      title="多格式同步，但不乱传"
      detail="剪贴板里可能是文本、图片，也可能是真文件。0.9.0 预览版把轻内容自动同步，大文件留给你手动确认。"
      width={820}
    />
    <div style={{position: "absolute", right: 120, top: 170, width: 760}}>
      <FormatRail iconName="text" title="文本" detail="秒级同步，支持去重，避免来回循环。" color={accent} delay={16} />
      <FormatRail iconName="image" title="图片" detail="截图和小图片自动传输，接收端生成记录。" color={green} delay={38} />
      <FormatRail iconName="file" title="小文件" detail="普通文件自动同步，目录和超限文件跳过提示。" color={amber} delay={60} />
      <FormatRail iconName="del" title="大文件" detail="不后台偷跑。需要时右键手动推送到设备。" color="#e45b5b" delay={82} />
    </div>
    <DeviceCard label="A 电脑" detail="复制客户资料.xlsx" type="pc" x={150} y={510} delay={28} active />
    <DeviceCard label="B 电脑" detail="记录列表出现文件" type="pc" x={560} y={510} delay={58} />
    <Packet fromX={500} fromY={620} toX={560} toY={620} color={amber} delay={70} />
  </SceneShell>
);

const FormatRail = ({
  iconName,
  title,
  detail,
  color,
  delay,
}: {
  iconName: string;
  title: string;
  detail: string;
  color: string;
  delay: number;
}) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        ...card,
        height: 104,
        marginBottom: 18,
        padding: "0 24px",
        display: "flex",
        alignItems: "center",
        gap: 18,
        opacity: soft(frame, delay, delay + 18, 0, 1),
        transform: `translateX(${soft(frame, delay, delay + 24, 44, 0)}px)`,
      }}
    >
      <div style={{width: 54, height: 54, borderRadius: 16, background: `${color}18`, display: "flex", alignItems: "center", justifyContent: "center"}}>
        <Img src={icon(iconName)} style={{width: 32, height: 32}} />
      </div>
      <div style={{width: 120, fontSize: 27, fontWeight: 950, color}}>{title}</div>
      <div style={{fontSize: 21, color: muted}}>{detail}</div>
    </div>
  );
};

const AndroidScene = () => (
  <SceneShell duration={sceneDurations.android} subtitle="安卓独立 App：快捷开关自动同步，也能一键推送到手机">
    <TitleBlock
      eyebrow="Mobile"
      title="手机不做输入法，也能接入"
      detail="独立安卓 App + 通知栏快捷磁贴：一个开关持续拉取电脑最新记录，一个开关立即推送到手机剪贴板。"
      width={760}
    />
    <div style={{position: "absolute", left: 130, bottom: 180, display: "flex", gap: 18}}>
      <Tile title="局域网自动同步" detail="开启前台服务，持续拉取最新记录" color={green} />
      <Tile title="推送到手机" detail="点击一次，把电脑最新记录写入手机剪贴板" color={accent} />
    </div>
    <div
      style={{
        ...card,
        position: "absolute",
        right: 170,
        top: 118,
        width: 430,
        height: 760,
        padding: 30,
        borderRadius: 46,
      }}
    >
      <div style={{height: 58, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 24, fontWeight: 950}}>
        ZSClip LAN
      </div>
      <div style={{height: 170, borderRadius: 28, background: "linear-gradient(135deg, #f7fbff, #e0f2ff)", padding: 22, marginTop: 18}}>
        <div style={{fontSize: 24, fontWeight: 930}}>当前连接</div>
        <div style={{fontSize: 21, color: accent, fontWeight: 900, marginTop: 18}}>办公电脑 · 已信任</div>
        <div style={{fontSize: 18, color: muted, marginTop: 8}}>192.168.1.18:38473</div>
      </div>
      <PhoneRecord title="最新记录已拉取" detail="客户回复模板" color={green} />
      <PhoneRecord title="剪贴板已更新" detail="来自 ZSClip PC" color={accent} />
      <div style={{fontSize: 19, color: muted, lineHeight: 1.55, marginTop: 28}}>
        Android 后台读取系统剪贴板有限制，所以本版主方向是“电脑推送到手机”。
      </div>
    </div>
  </SceneShell>
);

const Tile = ({title, detail, color}: {title: string; detail: string; color: string}) => (
  <div style={{...card, width: 350, height: 190, padding: 26}}>
    <div style={{width: 54, height: 54, borderRadius: 18, background: color, marginBottom: 20}} />
    <div style={{fontSize: 28, fontWeight: 950}}>{title}</div>
    <div style={{fontSize: 18, color: muted, marginTop: 10, lineHeight: 1.45}}>{detail}</div>
  </div>
);

const PhoneRecord = ({title, detail, color}: {title: string; detail: string; color: string}) => (
  <div style={{height: 94, borderRadius: 22, background: "#f7f9fc", padding: "16px 18px", marginTop: 18}}>
    <div style={{display: "flex", alignItems: "center", gap: 12}}>
      <div style={{width: 12, height: 12, borderRadius: "50%", background: color}} />
      <div style={{fontSize: 22, fontWeight: 900}}>{title}</div>
    </div>
    <div style={{fontSize: 18, color: muted, marginTop: 8}}>{detail}</div>
  </div>
);

const SafetyScene = () => (
  <SceneShell duration={sceneDurations.safety} subtitle="关闭后不驻留：socket、线程、轮询一起停">
    <TitleBlock
      eyebrow="Stable"
      title="同步要无感，也要可控"
      detail="开启才启动服务；关闭就释放资源。防火墙放行、token 信任、消息去重，都是为了减少打扰。"
      width={790}
    />
    <div style={{position: "absolute", right: 150, top: 210, display: "grid", gap: 22}}>
      <SafetyLine title="防火墙" detail="开启时尝试自动添加 TCP / UDP 入站规则" color={accent} />
      <SafetyLine title="信任 token" detail="配对后保存，陌生设备无法直接写入" color={green} />
      <SafetyLine title="消息去重" detail="origin + seq + hash 防止循环同步" color={amber} />
      <SafetyLine title="关闭即停止" detail="不用的时候不保留局域网后台轮询" color="#e45b5b" />
    </div>
  </SceneShell>
);

const SafetyLine = ({title, detail, color}: {title: string; detail: string; color: string}) => (
  <div style={{...card, width: 700, height: 100, padding: "0 26px", display: "flex", alignItems: "center", gap: 22}}>
    <div style={{width: 18, height: 58, borderRadius: 999, background: color}} />
    <div style={{width: 150, fontSize: 28, fontWeight: 950}}>{title}</div>
    <div style={{fontSize: 22, color: muted}}>{detail}</div>
  </div>
);

const CtaScene = () => {
  const frame = useCurrentFrame();
  return (
    <SceneShell duration={sceneDurations.cta} subtitle="ZSClip 0.9.0 多端同步预览版">
      <div style={{position: "absolute", inset: 0, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", paddingBottom: 80}}>
        <LogoMark size={136} />
        <div style={{fontSize: 78, fontWeight: 950, marginTop: 34}}>ZSClip 0.9.0</div>
        <div style={{fontSize: 42, fontWeight: 900, marginTop: 16}}>让复制内容，在设备之间自然流动</div>
        <div
          style={{
            ...card,
            marginTop: 48,
            height: 112,
            padding: "0 44px",
            display: "flex",
            alignItems: "center",
            gap: 28,
            transform: `translateY(${soft(frame, 10, 38, 38, 0)}px)`,
          }}
        >
          <div style={{fontSize: 27, fontWeight: 950}}>GitHub</div>
          <div style={{fontSize: 27, color: accent, fontWeight: 850}}>github.com/qiu7824/zsclip</div>
        </div>
      </div>
    </SceneShell>
  );
};

export const Zsclip09Preview = () => (
  <AbsoluteFill>
    <AudioTracks />
    <Sequence durationInFrames={sceneDurations.hero}>
      <HeroScene />
    </Sequence>
    <Sequence from={150} durationInFrames={sceneDurations.discover}>
      <DiscoverScene />
    </Sequence>
    <Sequence from={390} durationInFrames={sceneDurations.pairing}>
      <PairingScene />
    </Sequence>
    <Sequence from={660} durationInFrames={sceneDurations.modes}>
      <ModesScene />
    </Sequence>
    <Sequence from={930} durationInFrames={sceneDurations.formats}>
      <FormatsScene />
    </Sequence>
    <Sequence from={1200} durationInFrames={sceneDurations.android}>
      <AndroidScene />
    </Sequence>
    <Sequence from={1500} durationInFrames={sceneDurations.safety}>
      <SafetyScene />
    </Sequence>
    <Sequence from={1680} durationInFrames={sceneDurations.cta}>
      <CtaScene />
    </Sequence>
  </AbsoluteFill>
);

export const Zsclip09Cover = () => (
  <AbsoluteFill
    style={{
      color: ink,
      fontFamily: '"Microsoft YaHei UI", "Segoe UI", sans-serif',
    }}
  >
    <Backdrop />
    <div style={{position: "absolute", left: 138, top: 145}}>
      <LogoMark size={128} />
      <div style={{fontSize: 78, fontWeight: 950, marginTop: 42}}>ZSClip 0.9.0</div>
      <div style={{fontSize: 45, fontWeight: 930, marginTop: 16}}>多端同步预览版</div>
      <div style={{fontSize: 28, color: muted, marginTop: 26}}>
        局域网自动发现 · 设置页配对 · 安卓快捷开关
      </div>
    </div>
    <svg style={{position: "absolute", inset: 0, width: "100%", height: "100%"}}>
      <path d="M 1180 360 C 1060 470, 1030 560, 900 630" stroke={green} strokeWidth="8" fill="none" strokeLinecap="round" opacity="0.32" />
      <path d="M 1240 360 C 1370 455, 1390 540, 1515 620" stroke={accent} strokeWidth="8" fill="none" strokeLinecap="round" opacity="0.32" />
      <circle cx="1040" cy="485" r="14" fill={green} opacity="0.9" />
      <circle cx="1370" cy="475" r="14" fill={accent} opacity="0.9" />
    </svg>
    <DeviceCard label="办公电脑" detail="复制记录" type="pc" x={1000} y={130} delay={0} active />
    <DeviceCard label="家用电脑" detail="自动同步" type="pc" x={1335} y={500} delay={0} />
    <DeviceCard label="Android" detail="快捷拉取" type="phone" x={790} y={520} delay={0} />
  </AbsoluteFill>
);

import type { CSSProperties, ReactNode } from "react";
import {
  AbsoluteFill,
  Audio,
  Easing,
  interpolate,
  Sequence,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";

const red = "#c62828";
const black = "#050505";
const paper = "#f7f5ef";
const muted = "#aaa7a1";
const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

const sceneStyle: CSSProperties = {
  background: black,
  color: paper,
  fontFamily: "Geist, sans-serif",
  overflow: "hidden",
};

const mono: CSSProperties = { fontFamily: "Departure, monospace" };

const Reveal = ({
  children,
  delay = 0,
  distance = 50,
  style,
}: {
  children: ReactNode;
  delay?: number;
  distance?: number;
  style?: CSSProperties;
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = spring({
    frame: frame - delay,
    fps,
    config: { damping: 18, mass: 0.8, stiffness: 150 },
  });

  return (
    <div
      style={{
        opacity: enter,
        transform: `translateY(${(1 - enter) * distance}px)`,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

const Scene = ({
  children,
  dark = true,
  outAt,
}: {
  children: ReactNode;
  dark?: boolean;
  outAt?: number;
}) => {
  const frame = useCurrentFrame();
  const opacity = outAt
    ? interpolate(frame, [outAt - 8, outAt], [1, 0], clamp)
    : 1;

  return (
    <AbsoluteFill
      style={{
        ...sceneStyle,
        background: dark ? black : paper,
        color: dark ? paper : black,
        opacity,
      }}
    >
      {children}
    </AbsoluteFill>
  );
};

const CornerMark = ({ dark = true }: { dark?: boolean }) => (
  <div
    style={{
      ...mono,
      position: "absolute",
      top: 46,
      left: 58,
      color: dark ? paper : black,
      fontSize: 20,
      letterSpacing: 1.5,
    }}
  >
    wrec<span style={{ color: red }}>.</span>
  </div>
);

const Letterbox = () => (
  <>
    <div
      style={{
        position: "absolute",
        inset: "0 0 auto",
        height: 48,
        background: black,
        zIndex: 30,
      }}
    />
    <div
      style={{
        position: "absolute",
        inset: "auto 0 0",
        height: 48,
        background: black,
        zIndex: 30,
      }}
    />
  </>
);

const Grain = () => {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill
      style={{
        zIndex: 25,
        pointerEvents: "none",
        opacity: 0.13,
        mixBlendMode: "screen",
        backgroundImage:
          'url("data:image/svg+xml,%3Csvg viewBox=%270 0 180 180%27 xmlns=%27http://www.w3.org/2000/svg%27%3E%3Cfilter id=%27n%27%3E%3CfeTurbulence type=%27fractalNoise%27 baseFrequency=%27.88%27 numOctaves=%273%27 stitchTiles=%27stitch%27/%3E%3C/filter%3E%3Crect width=%27100%25%27 height=%27100%25%27 filter=%27url(%23n)%27 opacity=%27.42%27/%3E%3C/svg%3E")',
        transform: `translate(${(frame % 3) - 1}px, ${((frame * 7) % 3) - 1}px) scale(1.01)`,
      }}
    />
  );
};

const Intro = () => {
  const frame = useCurrentFrame();
  const cursor = interpolate(frame, [0, 72], [-240, 2100], {
    ...clamp,
    easing: Easing.inOut(Easing.cubic),
  });
  return (
    <Scene outAt={72}>
      <CornerMark />
      <div
        style={{
          position: "absolute",
          top: 0,
          left: cursor,
          width: 2,
          height: "100%",
          background: red,
          boxShadow: `0 0 70px 24px ${red}66`,
        }}
      />
      <div
        style={{ position: "absolute", left: 180, bottom: 170, width: 1540 }}
      >
        <Reveal delay={5}>
          <div
            style={{
              fontSize: 23,
              color: muted,
              letterSpacing: 4,
              textTransform: "uppercase",
            }}
          >
            screen recording / reconsidered
          </div>
        </Reveal>
        <Reveal delay={13} distance={80}>
          <div
            style={{
              fontSize: 98,
              fontWeight: 640,
              lineHeight: 0.98,
              letterSpacing: -5,
            }}
          >
            Your recorder should capture
            <br />
            <span style={{ color: red }}>the screen.</span> Not the machine.
          </div>
        </Reveal>
      </div>
      <Letterbox />
    </Scene>
  );
};

const ResourceMeter = ({
  label,
  value,
  width,
  delay,
}: {
  label: string;
  value: string;
  width: number;
  delay: number;
}) => {
  const frame = useCurrentFrame();
  const progress = interpolate(frame, [delay, delay + 55], [0, width], {
    ...clamp,
    easing: Easing.out(Easing.cubic),
  });
  return (
    <div style={{ marginBottom: 42 }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          fontSize: 22,
          ...mono,
        }}
      >
        <span>{label}</span>
        <span style={{ color: red }}>{value}</span>
      </div>
      <div style={{ height: 7, marginTop: 16, background: "#272727" }}>
        <div
          style={{
            height: "100%",
            width: `${progress}%`,
            background: red,
            boxShadow: `0 0 24px ${red}`,
          }}
        />
      </div>
    </div>
  );
};

const Problem = () => {
  const frame = useCurrentFrame();
  const drift = interpolate(frame, [0, 93], [0, -55], clamp);
  return (
    <Scene outAt={93}>
      <CornerMark />
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1.2fr 0.8fr",
          height: "100%",
          padding: "150px 180px 110px",
          gap: 150,
        }}
      >
        <div
          style={{ alignSelf: "center", transform: `translateY(${drift}px)` }}
        >
          <Reveal delay={0}>
            <div
              style={{ fontSize: 28, color: muted, marginBottom: 32, ...mono }}
            >
              THE OLD TRADE
            </div>
          </Reveal>
          <Reveal delay={7}>
            <div
              style={{
                fontSize: 105,
                lineHeight: 0.92,
                letterSpacing: -7,
                fontWeight: 650,
              }}
            >
              beautiful video.
              <br />
              <span style={{ color: "#656565" }}>brutal overhead.</span>
            </div>
          </Reveal>
        </div>
        <div
          style={{
            alignSelf: "center",
            borderLeft: "1px solid #333",
            paddingLeft: 70,
          }}
        >
          <ResourceMeter label="CPU" value="68%" width={88} delay={15} />
          <ResourceMeter label="MEMORY" value="1.4 GB" width={95} delay={23} />
          <ResourceMeter label="FANS" value="AUDIBLE" width={80} delay={31} />
          <div style={{ fontSize: 18, color: "#6e6e6e", ...mono }}>
            there is a better invariant.
          </div>
        </div>
      </div>
      <Letterbox />
    </Scene>
  );
};

const PreDrop = () => {
  const frame = useCurrentFrame();
  const command = "wrec record start --display 1 --fps 60 --codec hevc";
  const visible = command.slice(
    0,
    Math.floor(interpolate(frame, [10, 72], [0, command.length], clamp)),
  );
  const zoom = interpolate(frame, [0, 89], [1.03, 1], clamp);
  const scan = interpolate(frame, [0, 89], [0, 100], clamp);
  return (
    <Scene>
      <CornerMark />
      <div
        style={{
          position: "absolute",
          inset: 0,
          background: `linear-gradient(90deg, transparent ${scan - 0.2}%, ${red}22 ${scan}%, transparent ${scan + 0.2}%)`,
        }}
      />
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "grid",
          placeItems: "center",
          transform: `scale(${zoom})`,
        }}
      >
        <div style={{ width: 1320 }}>
          <div
            style={{
              display: "flex",
              gap: 10,
              padding: "16px 20px",
              background: "#161616",
              border: "1px solid #282828",
            }}
          >
            <span
              style={{
                width: 12,
                height: 12,
                borderRadius: 99,
                background: red,
              }}
            />
            <span
              style={{
                width: 12,
                height: 12,
                borderRadius: 99,
                background: "#464646",
              }}
            />
            <span
              style={{
                width: 12,
                height: 12,
                borderRadius: 99,
                background: "#464646",
              }}
            />
            <span
              style={{
                marginLeft: "auto",
                color: "#606060",
                fontSize: 14,
                ...mono,
              }}
            >
              agent session 001
            </span>
          </div>
          <div
            style={{
              height: 360,
              padding: "62px 52px",
              background: "#0b0b0b",
              border: "1px solid #282828",
              borderTop: 0,
              boxShadow: "0 60px 130px #000",
            }}
          >
            <div style={{ fontSize: 30, ...mono }}>
              <span style={{ color: red }}>› </span>
              {visible}
              <span style={{ opacity: frame % 18 < 10 ? 1 : 0, color: red }}>
                ▋
              </span>
            </div>
            {frame > 72 ? (
              <div
                style={{
                  fontSize: 23,
                  color: "#8f8f8f",
                  marginTop: 35,
                  ...mono,
                }}
              >
                <span style={{ color: red }}>●</span> ready — native pipeline
                armed
              </div>
            ) : null}
          </div>
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          bottom: 105,
          left: 0,
          right: 0,
          textAlign: "center",
          color: "#6d6d6d",
          fontSize: 17,
          letterSpacing: 4,
          ...mono,
        }}
      >
        WAIT FOR IT
      </div>
      <Letterbox />
    </Scene>
  );
};

const BeatFlash = ({ frames }: { frames: number[] }) => {
  const frame = useCurrentFrame();
  const strength = Math.max(
    ...frames.map((beat) =>
      interpolate(Math.abs(frame - beat), [0, 5], [0.33, 0], clamp),
    ),
  );
  return (
    <AbsoluteFill
      style={{
        background: paper,
        opacity: strength,
        zIndex: 20,
        mixBlendMode: "screen",
      }}
    />
  );
};

const Hero = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const hit = spring({
    frame,
    fps,
    config: { damping: 13, mass: 0.5, stiffness: 240 },
  });
  const push = interpolate(frame, [0, 144], [1.12, 0.92], clamp);
  const line = interpolate(frame, [0, 25], [0, 100], {
    ...clamp,
    easing: Easing.out(Easing.cubic),
  });
  return (
    <Scene>
      <AbsoluteFill style={{ background: red }} />
      <div
        style={{
          position: "absolute",
          top: 48,
          left: 0,
          width: `${line}%`,
          height: 2,
          background: black,
        }}
      />
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "grid",
          placeItems: "center",
          transform: `scale(${push}) rotate(${(1 - hit) * -3}deg)`,
        }}
      >
        <div style={{ position: "relative" }}>
          <div
            style={{
              fontSize: 440,
              lineHeight: 0.75,
              letterSpacing: -45,
              fontWeight: 900,
              color: black,
              transform: `translateY(${(1 - hit) * 220}px)`,
              ...mono,
            }}
          >
            wrec
          </div>
          <div
            style={{
              position: "absolute",
              right: 2,
              top: -28,
              color: paper,
              fontSize: 26,
              letterSpacing: 5,
              ...mono,
            }}
          >
            NEW DOCS / 2026
          </div>
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: 90,
          bottom: 90,
          fontSize: 22,
          color: black,
          ...mono,
        }}
      >
        THE EFFICIENT SCREEN RECORDER
      </div>
      <div
        style={{
          position: "absolute",
          right: 90,
          bottom: 90,
          fontSize: 22,
          color: black,
          ...mono,
        }}
      >
        MACOS / NATIVE / AGENT-READY
      </div>
      <BeatFlash frames={[0, 24, 44, 69, 97, 143]} />
      <Letterbox />
    </Scene>
  );
};

const PipelineNode = ({
  children,
  delay,
  accent = false,
}: {
  children: ReactNode;
  delay: number;
  accent?: boolean;
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = spring({
    frame: frame - delay,
    fps,
    config: { damping: 16, stiffness: 170 },
  });
  return (
    <div
      style={{
        opacity: enter,
        transform: `scale(${0.82 + enter * 0.18})`,
        border: `1px solid ${accent ? red : "#494949"}`,
        background: accent ? red : "#111",
        color: accent ? paper : paper,
        padding: "30px 34px",
        minWidth: 300,
        textAlign: "center",
        fontSize: 24,
        ...mono,
      }}
    >
      {children}
    </div>
  );
};

const NativePipeline = () => {
  const frame = useCurrentFrame();
  const travel = interpolate(frame % 30, [0, 30], [0, 1]);
  return (
    <Scene>
      <CornerMark />
      <div style={{ position: "absolute", top: 155, left: 180 }}>
        <Reveal>
          <div style={{ color: red, fontSize: 22, letterSpacing: 4, ...mono }}>
            NO BROWSER. NO ELECTRON. NO DETOUR.
          </div>
        </Reveal>
        <Reveal delay={8}>
          <div
            style={{
              fontSize: 102,
              lineHeight: 1,
              letterSpacing: -5,
              fontWeight: 650,
              marginTop: 20,
            }}
          >
            pixels take the direct route.
          </div>
        </Reveal>
      </div>
      <div
        style={{
          position: "absolute",
          left: 180,
          right: 180,
          top: 520,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <PipelineNode delay={8}>SCREEN</PipelineNode>
        <div
          style={{
            height: 1,
            flex: 1,
            background: "#424242",
            position: "relative",
          }}
        >
          <div
            style={{
              position: "absolute",
              top: -4,
              left: `${travel * 100}%`,
              width: 9,
              height: 9,
              borderRadius: 99,
              background: red,
              boxShadow: `0 0 20px ${red}`,
            }}
          />
        </div>
        <PipelineNode delay={21}>SCREENCAPTUREKIT</PipelineNode>
        <div
          style={{
            height: 1,
            flex: 1,
            background: "#424242",
            position: "relative",
          }}
        >
          <div
            style={{
              position: "absolute",
              top: -4,
              left: `${travel * 100}%`,
              width: 9,
              height: 9,
              borderRadius: 99,
              background: red,
              boxShadow: `0 0 20px ${red}`,
            }}
          />
        </div>
        <PipelineNode delay={44} accent>
          HEVC
        </PipelineNode>
      </div>
      <div
        style={{
          position: "absolute",
          left: 180,
          bottom: 140,
          display: "flex",
          gap: 90,
          color: "#8a8a8a",
          fontSize: 20,
          ...mono,
        }}
      >
        <span>hardware encoded</span>
        <span>60 fps</span>
        <span>native Swift + Rust</span>
      </div>
      <BeatFlash frames={[0, 21, 68, 89]} />
      <Letterbox />
    </Scene>
  );
};

const Metric = ({
  label,
  value,
  sub,
  delay,
}: {
  label: string;
  value: string;
  sub: string;
  delay: number;
}) => (
  <Reveal
    delay={delay}
    distance={80}
    style={{ borderTop: `2px solid ${red}`, paddingTop: 22 }}
  >
    <div style={{ fontSize: 22, color: "#7d7d7d", ...mono }}>{label}</div>
    <div
      style={{
        fontSize: 92,
        fontWeight: 650,
        letterSpacing: -5,
        marginTop: 12,
      }}
    >
      {value}
    </div>
    <div style={{ fontSize: 19, color: "#8b8b8b", marginTop: 8, ...mono }}>
      {sub}
    </div>
  </Reveal>
);

const Benchmarks = () => {
  const frame = useCurrentFrame();
  const ticker = interpolate(frame, [0, 120], [0, -480], clamp);
  return (
    <Scene>
      <CornerMark />
      <div style={{ position: "absolute", top: 152, left: 180, right: 180 }}>
        <Reveal>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 18,
              color: "#ababab",
              fontSize: 23,
              ...mono,
            }}
          >
            <span
              style={{
                display: "inline-grid",
                placeItems: "center",
                width: 36,
                height: 36,
                background: "#16321d",
                color: "#50df73",
              }}
            >
              ✓
            </span>
            RELEASE GATES / PASS
          </div>
        </Reveal>
        <Reveal delay={7}>
          <div
            style={{
              fontSize: 96,
              letterSpacing: -5,
              fontWeight: 650,
              marginTop: 24,
            }}
          >
            efficiency you can inspect.
          </div>
        </Reveal>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(4, 1fr)",
            gap: 48,
            marginTop: 110,
          }}
        >
          <Metric
            delay={12}
            label="EFFECTIVE FPS"
            value="29.49"
            sub="30 fps profile"
          />
          <Metric
            delay={21}
            label="PEAK RSS"
            value="45.5"
            sub="MiB process tree"
          />
          <Metric delay={44} label="DROPPED" value="0" sub="frames" />
          <Metric delay={68} label="FINALIZE" value="50" sub="milliseconds" />
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: ticker,
          right: 0,
          bottom: 70,
          whiteSpace: "nowrap",
          fontSize: 19,
          color: "#555",
          letterSpacing: 3,
          ...mono,
        }}
      >
        GROUND TRUTH DECODING · MACHINE-GUARDED RUNS · PUBLIC BENCHMARKS ·
        GROUND TRUTH DECODING · MACHINE-GUARDED RUNS · PUBLIC BENCHMARKS
      </div>
      <BeatFlash frames={[0, 21, 45, 68, 89]} />
      <Letterbox />
    </Scene>
  );
};

const DocCard = ({
  title,
  lines,
  delay,
}: {
  title: string;
  lines: number[];
  delay: number;
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = spring({
    frame: frame - delay,
    fps,
    config: { damping: 18, stiffness: 150 },
  });
  return (
    <div
      style={{
        height: 355,
        padding: 34,
        border: "1px solid #d5d1c7",
        background: "#fffefa",
        opacity: enter,
        transform: `translateY(${(1 - enter) * 110}px) rotate(${(1 - enter) * 2}deg)`,
        boxShadow: "0 28px 70px #3b1a1322",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          fontSize: 16,
          ...mono,
        }}
      >
        <span>{title}</span>
        <span style={{ color: red }}>↗</span>
      </div>
      <div
        style={{ height: 1, background: "#d9d6cf", margin: "25px 0 30px" }}
      />
      {lines.map((width, index) => (
        <div
          key={width + index}
          style={{
            height: index === 0 ? 14 : 8,
            width: `${width}%`,
            background: index === 0 ? black : "#c4c0b8",
            marginBottom: 15,
          }}
        />
      ))}
      <div
        style={{
          marginTop: 38,
          display: "inline-block",
          padding: "9px 12px",
          border: "1px solid #d2cec4",
          background: "#f1eee7",
          fontSize: 14,
          ...mono,
        }}
      >
        read the contract
      </div>
    </div>
  );
};

const Docs = () => (
  <Scene dark={false}>
    <CornerMark dark={false} />
    <div style={{ position: "absolute", left: 180, right: 180, top: 136 }}>
      <Reveal>
        <div style={{ fontSize: 21, color: red, letterSpacing: 5, ...mono }}>
          THE NEW DOCS
        </div>
      </Reveal>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "end",
          marginTop: 16,
        }}
      >
        <Reveal delay={6}>
          <div
            style={{
              fontSize: 94,
              lineHeight: 0.98,
              letterSpacing: -6,
              fontWeight: 650,
            }}
          >
            built for humans.
            <br />
            written for agents.
          </div>
        </Reveal>
        <Reveal delay={21}>
          <div
            style={{
              width: 410,
              color: "#494949",
              fontSize: 22,
              lineHeight: 1.5,
              marginBottom: 6,
            }}
          >
            Every command, event, configuration key, and architectural
            boundary—made explicit.
          </div>
        </Reveal>
      </div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: 28,
          marginTop: 80,
        }}
      >
        <DocCard
          delay={22}
          title="01 / AGENT DOCS"
          lines={[74, 93, 81, 66, 88]}
        />
        <DocCard
          delay={45}
          title="02 / ARCHITECTURE"
          lines={[62, 86, 92, 75, 69]}
        />
        <DocCard
          delay={68}
          title="03 / CONFIGURATION"
          lines={[81, 70, 94, 63, 83]}
        />
      </div>
    </div>
    <BeatFlash frames={[0, 21, 45, 68, 89]} />
    <Letterbox />
  </Scene>
);

const Manifesto = () => {
  const frame = useCurrentFrame();
  const words = ["native.", "efficient.", "obvious."];
  return (
    <Scene>
      <AbsoluteFill style={{ background: red }} />
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "grid",
          placeItems: "center",
        }}
      >
        <div style={{ display: "flex", alignItems: "baseline", gap: 45 }}>
          {words.map((word, index) => {
            const enter = spring({
              frame: frame - index * 20,
              fps: 30,
              config: { damping: 15, stiffness: 190 },
            });
            return (
              <div
                key={word}
                style={{
                  fontSize: 100,
                  letterSpacing: -6,
                  fontWeight: 700,
                  color: index === 2 ? paper : black,
                  opacity: enter,
                  transform: `translateY(${(1 - enter) * 100}px)`,
                  ...mono,
                }}
              >
                {word}
              </div>
            );
          })}
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          bottom: 95,
          left: 0,
          right: 0,
          textAlign: "center",
          fontSize: 20,
          color: black,
          letterSpacing: 4,
          ...mono,
        }}
      >
        RECORD FOR HOURS. NOT MINUTES.
      </div>
      <BeatFlash frames={[0, 22, 43]} />
      <Letterbox />
    </Scene>
  );
};

const Final = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = spring({
    frame,
    fps,
    config: { damping: 16, mass: 0.7, stiffness: 150 },
  });
  const underline = interpolate(frame, [18, 48], [0, 100], {
    ...clamp,
    easing: Easing.out(Easing.cubic),
  });
  return (
    <Scene>
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "grid",
          placeItems: "center",
        }}
      >
        <div
          style={{
            textAlign: "center",
            transform: `scale(${0.9 + enter * 0.1})`,
            opacity: enter,
          }}
        >
          <div
            style={{
              fontSize: 258,
              lineHeight: 0.8,
              letterSpacing: -28,
              fontWeight: 900,
              ...mono,
            }}
          >
            wrec<span style={{ color: red }}>.</span>
          </div>
          <div
            style={{
              fontSize: 30,
              color: "#aaa",
              marginTop: 52,
              letterSpacing: 6,
              ...mono,
            }}
          >
            THE NEW DOCS ARE LIVE
          </div>
          <div
            style={{
              position: "relative",
              display: "inline-block",
              marginTop: 34,
              fontSize: 45,
              ...mono,
            }}
          >
            wrec.app/docs
            <div
              style={{
                position: "absolute",
                left: 0,
                bottom: -12,
                width: `${underline}%`,
                height: 4,
                background: red,
              }}
            />
          </div>
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          bottom: 84,
          left: 0,
          right: 0,
          textAlign: "center",
          color: "#565656",
          fontSize: 17,
          letterSpacing: 3,
          ...mono,
        }}
      >
        OPEN SOURCE · MACOS 15+ · BUILT DIFFERENT
      </div>
      <Letterbox />
    </Scene>
  );
};

export const DocsRelease = () => (
  <AbsoluteFill style={sceneStyle}>
    <Audio src={staticFile("timeless-excerpt.mp3")} volume={0.92} />
    <Sequence durationInFrames={72}>
      <Intro />
    </Sequence>
    <Sequence from={72} durationInFrames={93}>
      <Problem />
    </Sequence>
    <Sequence from={165} durationInFrames={89}>
      <PreDrop />
    </Sequence>
    <Sequence from={254} durationInFrames={144}>
      <Hero />
    </Sequence>
    <Sequence from={398} durationInFrames={120}>
      <NativePipeline />
    </Sequence>
    <Sequence from={518} durationInFrames={120}>
      <Benchmarks />
    </Sequence>
    <Sequence from={638} durationInFrames={120}>
      <Docs />
    </Sequence>
    <Sequence from={758} durationInFrames={67}>
      <Manifesto />
    </Sequence>
    <Sequence from={825} durationInFrames={75}>
      <Final />
    </Sequence>
    <Grain />
  </AbsoluteFill>
);

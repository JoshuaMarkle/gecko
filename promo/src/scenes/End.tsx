// Scene 13: the wordmark on five keycaps (one press turns blue), the long name and the repo. The
// video ends on it.

import { AbsoluteFill } from "remotion";
import { clamp01, EASE, progress } from "../lib/motion";
import { C, KEY, MONO, WEIGHT } from "../lib/tokens";
import { useLayout } from "./common";

const T = { keys: 12, press: 65, sub: 40, url: 52 };
const WORD = "GECKO";
/** The key that gets pressed (blue). */
const PRESSED = 2;

/** The wordmark keycaps at `frame`; `glow` scales the pressed key's blue halo (the banner keeps it faint). */
export function Keycaps({ frame, glow: halo = 1 }: { frame: number; glow?: number }) {
  const s = 190, gap = 22;
  return (
    <div style={{ display: "flex", gap }}>
      {[...WORD].map((ch, i) => {
        const p = progress(frame, T.keys + i * 4, 22, EASE.out);
        const press = i === PRESSED ? clamp01(1 - Math.abs(frame - T.press - 4) / 5) : 0;
        const blue = i === PRESSED && frame >= T.press;
        const glow = blue ? Math.exp(-Math.max(0, frame - T.press) / 18) : 0;
        return (
          <div key={ch} style={{ width: s, height: s, borderRadius: 26, position: "relative", opacity: p,
            transform: `translateY(${(1 - p) * 60 + press * 10}px) scale(${1 - press * 0.03})`,
            background: blue ? C.accent : KEY.fill,
            boxShadow: `0 ${18 - press * 10}px 0 ${blue ? C.accentDeep : "#c9ccd3"}, 0 0 0 2px ${blue ? C.accent : C.hair}, 0 30px 60px #0000001f${blue ? `, 0 0 ${(80 + 120 * glow) * halo}px ${C.accent}${Math.round((120 + 100 * glow) * halo).toString(16).padStart(2, "0")}` : ""}`,
            display: "flex", alignItems: "center", justifyContent: "center" }}>
            {/* the dished top: a softer inner square */}
            <div style={{ position: "absolute", inset: 18, borderRadius: 18, background: blue ? "#1f78ff" : "#fafafa",
              boxShadow: blue ? "inset 0 -6px 14px #0003" : "inset 0 -6px 14px #0001" }} />
            <span style={{ position: "relative", fontSize: 112, fontWeight: WEIGHT.head, color: blue ? C.white : KEY.ink, lineHeight: 1 }}>{ch}</span>
          </div>
        );
      })}
    </div>
  );
}

export function End() {
  const { frame } = useLayout();
  const rise = (start: number) => { const p = progress(frame, start, 24, EASE.out); return { opacity: p, transform: `translateY(${(1 - p) * 20}px)` }; };
  return (
    <AbsoluteFill style={{ background: C.canvas, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 64,
      opacity: progress(frame, 0, 10, EASE.linear) }}>
      <Keycaps frame={frame} />
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 22, marginTop: 20 }}>
        <div style={{ fontSize: 50, fontWeight: WEIGHT.light, color: C.ink, ...rise(T.sub) }}>General Engine for Custom Keyboard Optimization</div>
        <div style={{ fontSize: 30, fontFamily: MONO, color: C.muted, ...rise(T.url) }}>github.com/JoshuaMarkle/gecko</div>
      </div>
    </AbsoluteFill>
  );
}

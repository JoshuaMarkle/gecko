// The README banner: the end card's settled keycaps (faint glow) and the long name on a rounded white card with a soft
// drop shadow; the margin around the card is transparent. `npx remotion still … banner` writes a PNG.

import { AbsoluteFill } from "remotion";
import { C, FONT, WEIGHT } from "./lib/tokens";
import { Keycaps } from "./scenes/End";

export const BANNER = { w: 1600, h: 600, margin: 40 } as const;

export function Banner() {
  return (
    <AbsoluteFill style={{ padding: BANNER.margin, fontFamily: FONT, WebkitFontSmoothing: "antialiased" }}>
      <div style={{ flex: 1, borderRadius: 36, background: C.canvas, boxShadow: "0 12px 28px #00000024, 0 2px 6px #00000014",
        display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 84 }}>
        <div style={{ transform: "scale(1.12)" }}><Keycaps frame={200} glow={0.25} /></div>
        <div style={{ fontSize: 46, fontWeight: WEIGHT.light, color: C.ink }}>General Engine for Custom Keyboard Optimization</div>
      </div>
    </AbsoluteFill>
  );
}

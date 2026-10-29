// The README banner: the end card's settled keycaps (faint glow) on a rounded white card with a soft
// drop shadow; the margin around the card is transparent. `npx remotion still … banner` writes a PNG.

import { AbsoluteFill } from "remotion";
import { C, FONT } from "./lib/tokens";
import { Keycaps } from "./scenes/End";

export const BANNER = { w: 1600, h: 520, margin: 40 } as const;

export function Banner() {
  return (
    <AbsoluteFill style={{ padding: BANNER.margin, fontFamily: FONT, WebkitFontSmoothing: "antialiased" }}>
      <div style={{ flex: 1, borderRadius: 36, background: C.canvas, boxShadow: "0 12px 28px #00000024, 0 2px 6px #00000014",
        display: "flex", alignItems: "center", justifyContent: "center" }}>
        <div style={{ transform: "scale(1.12) translateY(-8px)" }}><Keycaps frame={200} glow={0.25} /></div>
      </div>
    </AbsoluteFill>
  );
}

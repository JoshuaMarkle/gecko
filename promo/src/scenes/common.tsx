// Shared scene scaffolding: a white full-frame stage (fonts load in Root), `At` to place a child at a
// frame offset, and `useLayout` for frame-relative timing inside a scene.

import type { CSSProperties, ReactNode } from "react";
import { AbsoluteFill, Sequence, useCurrentFrame, useVideoConfig } from "remotion";
import { C, FONT } from "../lib/tokens";

export function Stage({ children, bg = C.canvas, style }: { children: ReactNode; bg?: string; style?: CSSProperties }) {
  return (
    <AbsoluteFill style={{ background: bg, fontFamily: FONT, color: C.ink, fontVariantNumeric: "tabular-nums", WebkitFontSmoothing: "antialiased", ...style }}>
      {children}
    </AbsoluteFill>
  );
}

/** Children see frame 0 at `from` (absolute frames of the parent). */
export function At({ from, dur, children }: { from: number; dur?: number; children: ReactNode }) {
  return <Sequence from={from} durationInFrames={dur} layout="none">{children}</Sequence>;
}

/** Frame and size helpers inside a scene. */
export function useLayout() {
  const frame = useCurrentFrame();
  const { width, height, fps, durationInFrames } = useVideoConfig();
  return { frame, width, height, fps, dur: durationInFrames, t: frame / fps };
}

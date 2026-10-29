// Display type: eyebrow labels, kinetic headlines (word or line rises), and the glyph glitch
// (after ../kinetic paragraphs' GLYPH GLITCH). Inter 650 / 450 / 300, sentence case, no tracking.

import { useCurrentFrame } from "remotion";
import { clamp01, DUR, EASE, presence, progress, STAGGER } from "../lib/motion";
import { hash01 } from "../lib/rng";
import { C, MONO, TYPE, WEIGHT } from "../lib/tokens";

/** Small muted label over a section: an optional index in mono, then the text. */
export function Eyebrow({ text, start = 0, exitAt, index, size = TYPE.eyebrow, color = C.muted }: {
  text: string; start?: number; exitAt?: number; index?: string; size?: number; color?: string;
}) {
  const frame = useCurrentFrame();
  const p = presence(frame, start, exitAt);
  const y = (1 - progress(frame, start, DUR.enter)) * 16;
  return (
    <div style={{ display: "flex", gap: 16, alignItems: "baseline", fontSize: size, fontWeight: WEIGHT.text, color, opacity: p, transform: `translateY(${y}px)` }}>
      {index && <span style={{ fontFamily: MONO, fontSize: size * 0.8, color: C.faint }}>{index}</span>}
      <span>{text}</span>
    </div>
  );
}

/**
 * A headline that rises in by word (or line): each unit slides up from under a mask, staggered.
 * `weight` 650 for statements, 300 for leads. Exits by fading up.
 */
export function KineticHeadline({ text, start = 0, exitAt, by = "word", size = TYPE.headline, weight = WEIGHT.head, color = C.ink, align = "left", width }: {
  text: string; start?: number; exitAt?: number; by?: "word" | "line"; size?: number; weight?: number; color?: string;
  align?: "left" | "center"; width?: number;
}) {
  const frame = useCurrentFrame();
  const units = by === "word" ? text.split(" ") : text.split("\n");
  const out = exitAt === undefined ? 0 : progress(frame, exitAt, DUR.fast + 4, EASE.in);
  return (
    <div style={{ fontSize: size, fontWeight: weight, lineHeight: 1.05, color, textAlign: align, width, display: "flex", flexWrap: "wrap",
      justifyContent: align === "center" ? "center" : "flex-start", columnGap: by === "word" ? size * 0.26 : 0,
      opacity: 1 - out, transform: `translateY(${-out * 20}px)` }}>
      {units.map((w, i) => {
        const p = progress(frame, start + i * (by === "word" ? STAGGER.word : STAGGER.line), DUR.move);
        return (
          <span key={i} style={{ display: "inline-block", overflow: "hidden", paddingBottom: size * 0.08, flexBasis: by === "line" ? "100%" : undefined }}>
            <span style={{ display: "inline-block", transform: `translateY(${(1 - p) * 105}%)` }}>{w}</span>
          </span>
        );
      })}
    </div>
  );
}

/** Glyphs a glitch cycles through (defaults to the prose corpus's own symbols). */
export const GLITCH_SET = "abcdefghijklmnopqrstuvwxyz,./;'-0123456789()[]{}<>=+*&|!?:\"_#$%^~`@";

/**
 * Text whose characters glitch through `charset` and settle left to right (`glitch` 0 → settled,
 * 1 → all noise), optionally lifting off the line (`lift` 0..1: seeded z-up offset and rotation).
 * Deterministic per (seed, frame).
 */
export function GlitchText({ text, seed = 1, glitch = 0, lift = 0, charset = GLITCH_SET, size = TYPE.body, weight = WEIGHT.text, color = C.ink, mono = false, rate = 2 }: {
  text: string; seed?: number; glitch?: number; lift?: number; charset?: string; size?: number; weight?: number; color?: string; mono?: boolean;
  /** Frames per glyph change. */
  rate?: number;
}) {
  const frame = useCurrentFrame();
  const tick = Math.floor(frame / rate);
  const n = text.length;
  return (
    <span style={{ fontSize: size, fontWeight: weight, color, fontFamily: mono ? MONO : undefined, whiteSpace: "pre-wrap" }}>
      {[...text].map((ch, i) => {
        // Each character settles once `glitch` drops below its threshold (left settles first).
        const threshold = (i / Math.max(1, n)) * 0.8 + hash01(seed, i) * 0.2;
        const noisy = ch !== " " && glitch > threshold;
        const g = noisy ? charset[Math.floor(hash01(seed + tick, i) * charset.length)] : ch;
        const l = clamp01(lift * 1.4 - hash01(seed * 7, i) * 0.4);
        const dy = -l * (40 + hash01(seed * 3, i) * 120), rot = (hash01(seed * 5, i) - 0.5) * 60 * l;
        return (
          <span key={i} style={{ display: "inline-block", whiteSpace: "pre", transform: l ? `translateY(${dy}px) rotate(${rot}deg)` : undefined,
            opacity: 1 - l * 0.3, color: noisy ? C.muted : color }}>{g}</span>
        );
      })}
    </span>
  );
}

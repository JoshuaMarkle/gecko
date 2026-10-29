// "2D site, 3D camera": flat product panels placed on planes in one CSS 3D space, so text stays
// crisp. A camera (position + yaw/pitch) moves through them; depth fades panels into the black.
// The tunnel lines four walls with panels and flies the camera down it.

import type { ReactNode } from "react";
import { clamp01 } from "../lib/motion";
import { C, H, HAIRLINE, R, W } from "../lib/tokens";
import { Trace } from "./trace";

export interface Cam3D { x?: number; y?: number; z?: number; yaw?: number; pitch?: number; roll?: number; perspective?: number }

/**
 * A 3D space the size of the frame. Children are positioned in world px (x right, y down, z toward
 * the viewer); the camera sits at (x, y, z) looking down −z.
 */
export function Space3D({ cam = {}, children, width = W, height = H }: { cam?: Cam3D; children: ReactNode; width?: number; height?: number }) {
  const { x = 0, y = 0, z = 0, yaw = 0, pitch = 0, roll = 0, perspective = 1400 } = cam;
  return (
    <div style={{ position: "absolute", inset: 0, perspective, perspectiveOrigin: "50% 50%", overflow: "hidden" }}>
      <div style={{ position: "absolute", left: width / 2, top: height / 2, transformStyle: "preserve-3d",
        transform: `rotateZ(${-roll}deg) rotateX(${-pitch}deg) rotateY(${-yaw}deg) translate3d(${-x}px, ${-y}px, ${-z}px)` }}>
        {children}
      </div>
    </div>
  );
}

/**
 * A flat panel on a plane in the space, centred at (x, y, z), turned by (rx, ry, rz) degrees.
 * `lift` raises it toward the viewer (px); `fog` 0..1 fades it into the stage with depth (pass the panel's
 * distance fraction); `shadow` adds a soft drop under lifted panels.
 */
export function FloatingPanel({ x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0, lift = 0, opacity = 1, fog = 0, scale = 1, shadow = 0, children }: {
  x?: number; y?: number; z?: number; rx?: number; ry?: number; rz?: number; lift?: number; opacity?: number; fog?: number; scale?: number; shadow?: number;
  children: ReactNode;
}) {
  return (
    <div style={{ position: "absolute", left: 0, top: 0, transformStyle: "preserve-3d",
      transform: `translate3d(${x}px, ${y}px, ${z + lift}px) rotateX(${rx}deg) rotateY(${ry}deg) rotateZ(${rz}deg) scale(${scale})`, opacity }}>
      <div style={{ transform: "translate(-50%, -50%)", position: "relative",
        filter: shadow > 0 ? `drop-shadow(0 ${24 * shadow}px ${48 * shadow}px rgba(0,0,0,${0.16 * shadow}))` : undefined }}>
        {children}
        {fog > 0 && <div style={{ position: "absolute", inset: -4, background: C.canvas, opacity: clamp01(fog) }} />}
      </div>
    </div>
  );
}

/** A plain card frame for panels in space (panel fill, hairline edge, --r-md). */
export function SpaceCard({ w, h, children, pad = 24 }: { w: number; h: number; children: ReactNode; pad?: number }) {
  return (
    <div style={{ width: w, height: h, background: C.panel, border: `${HAIRLINE}px solid ${C.hair}`, borderRadius: R.md, padding: pad,
      boxSizing: "border-box", overflow: "hidden" }}>
      {children}
    </div>
  );
}

/**
 * Tunnel: `panels` in pairs (left, right) along its length, one pair every `spacing` px, facing the camera and mirrored about the axis. `progress` 0..1 flies the camera from
 * the mouth to the end. Panels fade in from the white far ahead and are culled once passed. No cards:
 * the charts float on the stage.
 */
export function Tunnel({ panels, progress, spacing = 900, panelW = 600, panelH = 600, gap = 540 }: {
  panels: ReactNode[]; progress: number; spacing?: number; panelW?: number; panelH?: number;
  /** Clear space between a pair (px), centred on the axis. */
  gap?: number;
}) {
  const stations = Math.ceil(panels.length / 2);
  const length = (stations + 2) * spacing;
  const perspective = 900;
  const camZ = -progress * length;
  const eyeZ = camZ + perspective; // CSS perspective puts the eye this far in front of the camera plane
  return (
    <Space3D cam={{ z: camZ, perspective }}>
      {panels.map((p, i) => {
        const z = -(Math.floor(i / 2) + 1) * spacing;
        const dist = eyeZ - z;
        if (dist < 60) return null; // passed
        const fog = clamp01((dist - spacing * 1.7) / (spacing * 1.5));
        const x = (i % 2 ? 1 : -1) * (gap / 2 + panelW / 2);
        return (
          <FloatingPanel key={i} x={x} y={0} z={z} fog={fog}>
            <div style={{ width: panelW, height: panelH, display: "flex", alignItems: "center", justifyContent: "center" }}>{p}</div>
          </FloatingPanel>
        );
      })}
    </Space3D>
  );
}

/**
 * A callout pinned to a projected 3D point: a copper pad on the point, a PCB trace (45° turns) to
 * a via at the label, the value in 650. `reveal` draws the trace then the text; `lead` sets where
 * the trace turns, so callouts sharing a point fan out like a bus.
 */
export function StatCallout({ at, to, label, value, reveal = 1, align = "left", lead = 0.4 }: {
  at: [number, number]; to: [number, number]; label: string; value: string; reveal?: number; align?: "left" | "right"; lead?: number;
}) {
  const t = clamp01(reveal);
  const txt = clamp01(t * 2 - 1);
  return (
    <>
      <svg style={{ position: "absolute", inset: 0, overflow: "visible", pointerEvents: "none" }} width={W} height={H}>
        <Trace from={at} to={to} reveal={t * 2} lead={lead} tail={40} />
      </svg>
      <div style={{ position: "absolute", left: align === "left" ? to[0] + 16 : undefined, right: align === "right" ? W - to[0] + 16 : undefined,
        top: to[1] - 30, opacity: txt, transform: `translateY(${(1 - txt) * 8}px)`, textAlign: align }}>
        <div style={{ fontSize: 44, fontWeight: 650, lineHeight: 1 }}>{value}</div>
        <div style={{ fontSize: 24, color: C.muted, marginTop: 6 }}>{label}</div>
      </div>
    </>
  );
}

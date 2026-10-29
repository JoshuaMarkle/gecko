// The component sheet (`npm run components`): the promo's 2D components as they stand, and a 3D
// viewer to orbit the models by hand. Remotion's `Thumbnail` supplies the frame context the
// components read (a single still frame, no playback).

import { Thumbnail } from "@remotion/player";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import { loadFonts } from "../lib/font";
import { C, FONT, FPS, MONO } from "../lib/tokens";
import { Stage } from "../scenes/common";
import { ITEMS, ModelScene, MODELS } from "./entries";

loadFonts();

/** Late enough that every build-in (headlines, badges, the end card) has settled. */
const SETTLED = 300;

const Still2D = ({ i }: { i: number }) => <Stage>{ITEMS[i].el()}</Stage>;
const View3D = ({ i }: { i: number }) => <Stage><ModelScene m={MODELS[i]} /></Stage>;

const tab = (on: boolean) => ({
  font: `500 13px ${FONT}`, padding: "6px 14px", border: `1px solid ${on ? C.ink : C.hair}`, borderRadius: 6, cursor: "pointer",
  background: on ? C.ink : C.canvas, color: on ? C.white : C.ink,
});

function Sheet2D() {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(420px, 1fr))", gap: 16 }}>
      {ITEMS.map((it, i) => (
        <section key={it.title} style={{ gridColumn: it.wide ? "1 / -1" : undefined, background: C.canvas, border: `1px solid ${C.hair}`, borderRadius: 10, padding: 14 }}>
          <div style={{ display: "flex", gap: 10, alignItems: "baseline", marginBottom: 10 }}>
            <h3 style={{ margin: 0, fontSize: 14, fontWeight: 600 }}>{it.title}</h3>
            <code style={{ fontFamily: MONO, fontSize: 11, color: C.muted }}>src/{it.file}</code>
          </div>
          <Thumbnail component={Still2D} inputProps={{ i }} compositionWidth={it.w} compositionHeight={it.h} durationInFrames={SETTLED + 1} fps={FPS}
            frameToDisplay={SETTLED} style={{ width: it.wide ? `min(100%, ${Math.round(520 * it.w / it.h)}px)` : "100%", margin: "0 auto", display: "block" }} />
        </section>
      ))}
    </div>
  );
}

function Viewer3D() {
  const [i, setI] = useState(0);
  const m = MODELS[i];
  return (
    <div style={{ display: "flex", gap: 16, height: "calc(100vh - 110px)" }}>
      <div style={{ width: 200, display: "flex", flexDirection: "column", gap: 6 }}>
        {MODELS.map((x, j) => <button key={x.title} style={{ ...tab(j === i), textAlign: "left" }} onClick={() => setI(j)}>{x.title}</button>)}
        <div style={{ fontSize: 12, color: C.muted, marginTop: 10, lineHeight: 1.5 }}>Drag to orbit<br />Scroll to zoom<br />Right-drag to pan</div>
        <code style={{ fontFamily: MONO, fontSize: 11, color: C.muted, marginTop: 6 }}>src/{m.file}</code>
      </div>
      <div style={{ flex: 1, minWidth: 0, display: "flex", alignItems: "center", justifyContent: "center", background: C.canvas, border: `1px solid ${C.hair}`, borderRadius: 10, overflow: "hidden" }}>
        {/* keyed by model: one WebGL context at a time, and each model starts at its own view */}
        <Thumbnail key={i} component={View3D} inputProps={{ i }} compositionWidth={1600} compositionHeight={900} durationInFrames={1} fps={FPS}
          frameToDisplay={0} style={{ width: "100%", maxHeight: "100%", aspectRatio: "16 / 9" }} />
      </div>
    </div>
  );
}

function App() {
  const [view, setView] = useState<"2D" | "3D">(location.hash === "#3d" ? "3D" : "2D");
  const go = (v: "2D" | "3D") => { setView(v); history.replaceState(null, "", v === "3D" ? "#3d" : "#"); };
  return (
    <div style={{ fontFamily: FONT, color: C.ink, background: C.soft, minHeight: "100vh", padding: 20, boxSizing: "border-box" }}>
      <header style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 20 }}>
        <div style={{ fontWeight: 700, fontSize: 18, letterSpacing: 1 }}>GECKO <span style={{ fontWeight: 400, color: C.muted, fontSize: 14, letterSpacing: 0 }}>promo components</span></div>
        <span style={{ flex: 1 }} />
        <button style={tab(view === "2D")} onClick={() => go("2D")}>2D components</button>
        <button style={tab(view === "3D")} onClick={() => go("3D")}>3D models</button>
      </header>
      {view === "2D" ? <Sheet2D /> : <Viewer3D />}
    </div>
  );
}

document.body.style.margin = "0";
// No StrictMode: its double effects dispose Hands3D's rigged meshes (Skins) before they draw.
createRoot(document.getElementById("root")!).render(<App />);

// 2D layout editor (frontend.md "2D and 3D board representations"): click two keys to swap
// them, Pin mode locks a key's symbol, and a layer picker shows each layer of a layered layout.
// Opened from Search (Edit layout) as an overlay on the main area.

import { useState } from "react";
import type { Analysis, Geometry } from "../api";
import { heatColor, heatText, INK } from "../colors";
import { EMPTY, fmt, layerKey, layerSlots } from "../format";
import { FitBoard } from "./Board2D";
import { Seg } from "./Form";

interface Props {
  geometry: Geometry;
  /** Fitted, slot-ordered layout being edited. */
  chars: string;
  label: string;
  a: Analysis | null;
  pins: string;
  setPins: (p: string) => void;
  onSwap: (i: number, j: number) => void;
  canUndo: boolean;
  onUndo: () => void;
  onClose: () => void;
}

export function LayoutEditor(p: Props) {
  const [mode, setMode] = useState<"swap" | "pin">("swap");
  const [layer, setLayer] = useState(0);
  const [heat, setHeat] = useState(false);
  /** Selected slot (an index into the flat layered string, not a key): survives layer changes. */
  const [sel, setSel] = useState<number | null>(null);
  const sym = [...p.chars];
  const slots = layerSlots(p.geometry, p.chars);
  const shown = Math.min(layer, slots.length - 1);
  const pinned = new Set([...p.pins.replace(/\s/g, "")]);
  const maxHeat = Math.max(...(p.a?.heat ?? [1]), 1e-9);

  const click = (k: number) => {
    const i = slots[shown][k];
    if (i === null) return;
    if (mode === "pin") {
      const c = sym[i];
      if (!c || c === EMPTY) return;
      p.setPins(pinned.has(c) ? [...pinned].filter((x) => x !== c).join("") : p.pins.replace(/\s/g, "") + c);
      return;
    }
    if (pinned.has(sym[i])) return;
    if (sel === null) { setSel(i); return; }
    if (sel !== i) p.onSwap(sel, i);
    setSel(null);
  };

  return (
    <div className="editor">
      <div className="editor-head">
        <b>Edit layout</b><span className="muted">{p.label}{p.a ? ` · ${fmt(p.a.breakdown.score, 3)}` : ""}</span>
        <span className="grow" />
        {slots.length > 1 && (
          <Seg inline value={`${shown}`} onChange={(v) => setLayer(+v)}
            options={slots.map((_, l) => [`${l}`, l === 0 ? "Base" : `${layerKey(sym.slice(0, slots[0].length), l)?.glyph ?? ""} Layer ${l}`] as [string, string])} />
        )}
        <Seg inline value={mode} onChange={(m) => { setMode(m); setSel(null); }} options={[["swap", "Swap"], ["pin", "Pin"]]} />
        <div className="seg inline"><button className={heat ? "on" : ""} onClick={() => setHeat(!heat)} title="Shade keys by keystroke share">Heat</button></div>
        <button className="btn soft small" disabled={!p.canUndo} onClick={p.onUndo}>Undo edits</button>
        <button className="btn primary small" onClick={p.onClose}>Done</button>
      </div>
      <div className="editor-board">
        <FitBoard geometry={p.geometry} max={72} pad={24} onKey={click} cells={(k) => {
          const i = slots[shown][k];
          if (i === null) return { fill: "#ffffff", text: INK, main: "", dim: true, title: "Not on this layer (the finger holding the layer key)" };
          const c = sym[i] ?? "";
          const t = heat && p.a ? (p.a.heat[i] ?? 0) / maxHeat : 0;
          const pin = pinned.has(c);
          return {
            fill: heat ? heatColor(t) : pin ? "#ececec" : "#ffffff",
            text: heat ? heatText(t) : INK,
            main: c === EMPTY ? "" : c.toUpperCase(),
            sub: heat && p.a ? fmt(p.a.heat[i] ?? 0, 1) : undefined,
            mark: pin ? "pin" : undefined,
            ring: sel === i,
            title: pin ? `${c} is pinned: it keeps this key during optimization` : undefined,
          };
        }} />
      </div>
      <div className="editor-foot">
        <span className="muted">
          {mode === "pin" ? "Click a key to pin or unpin its symbol." : sel !== null ? `Swap ${(sym[sel] ?? "").toUpperCase()} with… (click another key${slots.length > 1 ? ", on any layer" : ""})` : "Click two keys to swap them. Pinned keys stay put."}
        </span>
        <span className="grow" />
        <span className="muted">Pins <span className="mono">{[...pinned].join("") || "none"}</span></span>
        {pinned.size > 0 && <button className="btn soft small" onClick={() => p.setPins("")}>Clear pins</button>}
      </div>
    </div>
  );
}

// Language composition (Language tab main view; web-screens.md "Language"): a recipe's parts as
// one stacked bar, then every symbol's share split by the part it comes from, in two charts
// (letters, space and keys | punctuation and digits). Weights can be edited, parts added or
// removed and Backspace changed; the charts preview live (mix.ts) and Save builds the new
// language on the server, ready for runs.

import { useEffect, useMemo, useState } from "react";
import { api, type Composition, type CorpusInfo } from "../api";
import { fmt } from "../format";
import { byShare, notInCharset, OTHER, partColor, preview, sameWeights, type PartShares, type SymbolGroup } from "../mix";
import { niceTicks, useSize } from "./Charts";

interface Props {
  corpus: CorpusInfo;
  corpora: CorpusInfo[];
  /** A language was saved: reload the list and select it. */
  onSaved: (id: string) => void;
  onOpen: (id: string) => void;
}

interface Draft { parts: { corpus: string; weight: number }[]; edits: number }

const HATCH = "mixhatch";
const label = (s: string) => (s === " " ? "␣" : s);
const pm = (v: number) => `${fmt(1000 * v, v >= 0.01 ? 1 : v >= 0.001 ? 2 : 3)} ‰`;

function Hatch() {
  return (
    <defs>
      <pattern id={HATCH} width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
        <rect width="5" height="5" fill="#ffffff" />
        <line x1="0" y1="0" x2="0" y2="5" stroke={OTHER} strokeWidth="2.5" />
      </pattern>
    </defs>
  );
}

const fillOf = (i: number, n: number) => (i === n ? `url(#${HATCH})` : partColor(i));

/** The parts' shares of all keystrokes as one horizontal bar (2px gaps), labels where they fit. */
function MixBar({ names, shares }: { names: string[]; shares: number[] }) {
  const [box, { w }] = useSize<HTMLDivElement>();
  const h = 28;
  let x = 0;
  const n = names.length;
  return (
    <div ref={box} className="mixbar">
      <svg width={w} height={h}>
        <Hatch />
        {shares.map((s, i) => {
          if (s <= 0) return null;
          const bw = s * w;
          const x0 = x;
          x += bw;
          const name = i === n ? "modelled" : names[i];
          return (
            <g key={i}>
              <title>{`${name}: ${fmt(100 * s, 1)}% of keystrokes`}</title>
              <rect x={x0} y={0} width={Math.max(1, bw - 2)} height={h} rx={4} fill={fillOf(i, n)} />
              {bw > 70 && <text x={x0 + 8} y={h / 2 + 4} className="ink">{`${name} ${fmt(100 * s, 0)}%`}</text>}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

/** Stacked columns: one per symbol, height = its share, segments = its parts. */
function SymbolBars({ syms, mono, contrib, names, title }: { syms: string[]; mono: Record<string, number>; contrib: Record<string, number[]>; names: string[]; title: string }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const [hover, setHover] = useState<{ s: string; x: number; y: number } | null>(null);
  const m = { l: 40, r: 4, t: 6, b: 18 };
  const max = Math.max(...syms.map((s) => mono[s]), 1e-9);
  const ticks = niceTicks(0, 1000 * max, 3);
  const top = Math.max(ticks[ticks.length - 1] / 1000, max);
  const ih = h - m.t - m.b;
  const bw = (w - m.l - m.r) / Math.max(syms.length, 1);
  const y = (v: number) => m.t + ih - (v / top) * ih;
  const n = names.length;
  return (
    <div className="mixchart">
      <h4>{title} <span className="hint">‰ of keystrokes, by part</span></h4>
      <div ref={box} className="chartfill" onMouseLeave={() => setHover(null)}>
        <svg width={w} height={h}>
          <Hatch />
          {ticks.map((t) => (
            <g key={t}>
              <line x1={m.l} x2={w - m.r} y1={y(t / 1000)} y2={y(t / 1000)} stroke="var(--hair-soft)" />
              <text x={m.l - 6} y={y(t / 1000) + 4} textAnchor="end">{+t.toFixed(2)}</text>
            </g>
          ))}
          {syms.map((s, i) => {
            const x = m.l + i * bw;
            let acc = 0;
            const segs = (contrib[s] ?? []).map((v, k) => {
              const y0 = y(acc + v), y1 = y(acc);
              acc += v;
              return v > 0 && y1 - y0 > 0.3 ? <rect key={k} x={x + 1} y={y0} width={Math.max(1, bw - 2)} height={Math.max(0.5, y1 - y0 - (y1 - y0 > 3 ? 1 : 0))} fill={fillOf(k, n)} /> : null;
            });
            return (
              <g key={s} onMouseMove={(e) => setHover({ s, x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY })}>
                <rect x={x} y={m.t} width={bw} height={ih + m.b} fill="transparent" />
                {mono[s] === 0 && <line x1={x + 2} x2={x + bw - 2} y1={m.t + ih} y2={m.t + ih} stroke="var(--error)" strokeWidth={2} />}
                {segs}
                {bw >= 7 && <text x={x + bw / 2} y={h - 5} textAnchor="middle" className={hover?.s === s ? "ink mono" : "mono"}>{label(s)}</text>}
              </g>
            );
          })}
        </svg>
        {hover && (
          <div className="tip" style={{ left: Math.max(0, Math.min(hover.x + 12, w - 230)), top: Math.max(0, Math.min(hover.y - 10, h - 120)) }}>
            <b className="mono">{label(hover.s)}</b> {pm(mono[hover.s])}
            {(contrib[hover.s] ?? []).map((v, k) => v > 0 && (
              <div key={k}><span className="swatch" style={{ background: k === n ? OTHER : partColor(k) }} />{k === n ? "modelled" : names[k]} {pm(v)}</div>
            ))}
            {mono[hover.s] === 0 && <div className="muted">never typed in this language</div>}
          </div>
        )}
      </div>
    </div>
  );
}

export function LanguageMix({ corpus, corpora, onSaved, onOpen }: Props) {
  const [comp, setComp] = useState<Composition | null>(null);
  const [extra, setExtra] = useState<Record<string, Record<string, number>>>({});
  const [err, setErr] = useState<string | null>(null);
  const saved: Draft = useMemo(() => ({
    parts: corpus.recipe?.parts.map((p) => ({ corpus: p.corpus, weight: p.weight })) ?? [{ corpus: corpus.id, weight: 1 }],
    edits: corpus.recipe?.edits ?? 0,
  }), [corpus]);
  const [draft, setDraft] = useState<Draft>(saved);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    setComp(null); setErr(null); setDraft(saved); setName("");
    api.composition(corpus.id).then((c) => alive && setComp(c)).catch((e) => alive && setErr((e as Error).message));
    return () => { alive = false; };
  }, [corpus.id, saved]);

  // Part monograms: from the composition, else fetched when a part is added.
  const monoOf = (id: string) => comp?.parts.find((p) => p.corpus === id)?.mono ?? extra[id];
  useEffect(() => {
    for (const p of draft.parts) {
      if (comp && !monoOf(p.corpus)) {
        api.composition(p.corpus).then((c) => setExtra((x) => ({ ...x, [p.corpus]: c.mono }))).catch((e) => setErr((e as Error).message));
      }
    }
  }, [draft.parts, comp]); // eslint-disable-line react-hooks/exhaustive-deps

  const edited = !sameWeights(draft.parts, saved.parts) || Math.abs(draft.edits - saved.edits) > 1e-9;
  const view = useMemo(() => {
    if (!comp) return null;
    const parts: PartShares[] = draft.parts.flatMap((p) => { const mono = monoOf(p.corpus); return mono ? [{ corpus: p.corpus, share: p.weight, mono }] : []; });
    if (parts.length !== draft.parts.length) return null;
    const pv = preview(parts, draft.edits, edited ? 0 : comp.nav);
    // An unchanged recipe shows the built language's exact shares; the split stays proportional.
    if (!edited) {
      for (const s of Object.keys(comp.mono)) {
        const c = pv.contrib[s] ?? [];
        const t = c.reduce((a, b) => a + b, 0);
        pv.contrib[s] = t > 0 ? c.map((v) => (v * comp.mono[s]) / t) : c;
        pv.mono[s] = comp.mono[s];
      }
    }
    return pv;
  }, [comp, draft, extra, edited]); // eslint-disable-line react-hooks/exhaustive-deps

  if (err) return <div className="pad"><div className="notice error">{err}</div></div>;
  if (!comp || !view) return <div className="empty">Loading composition…</div>;

  const names = draft.parts.map((p) => p.corpus);
  const charset = Object.keys(view.mono).join("");
  const missing = notInCharset(charset);
  const zero = Object.keys(view.mono).filter((s) => view.mono[s] === 0);
  const tot = draft.parts.reduce((a, p) => a + p.weight, 0) || 1;
  const choices = corpora.filter((c) => c.id !== corpus.id && !draft.parts.some((p) => p.corpus === c.id));
  const setPart = (i: number, weight: number) => setDraft({ ...draft, parts: draft.parts.map((p, k) => (k === i ? { ...p, weight } : p)) });
  const bundled = corpora.some((c) => c.id === name.trim());
  const save = async () => {
    setBusy(true);
    try {
      const r = await api.saveMix({ name: name.trim(), parts: draft.parts, edits: draft.edits, description: `Mix of ${draft.parts.map((p) => `${p.corpus} ${fmt((100 * p.weight) / tot, 0)}%`).join(", ")}` });
      onSaved(r.id);
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="langmix">
      <div className="mixhead">
        <b>{corpus.name}</b>
        <span className="status">{corpus.kind}</span>
        {edited && <span className="badge warn">preview</span>}
        <span className="muted mixdesc">{corpus.description}</span>
      </div>
      <div className="mixbody">
        <div className="mixparts">
          <h4>Parts <span className="hint">weight · share of keystrokes</span></h4>
          {draft.parts.map((p, i) => (
            <div key={p.corpus} className="mixpart">
              <span className="swatch" style={{ background: partColor(i) }} />
              <button className="link mono" title={`Open ${p.corpus}`} onClick={() => onOpen(p.corpus)}>{p.corpus}</button>
              <input type="range" min={1} max={100} step={1} value={Math.round((100 * p.weight) / tot)} aria-label={`${p.corpus} weight`}
                onChange={(e) => setPart(i, (+e.target.value / 100) * tot)} />
              <span className="num">{fmt(100 * view.parts[i], 1)}%</span>
              {draft.parts.length > 1 && <button className="x" aria-label={`Remove ${p.corpus}`} title="Remove part" onClick={() => setDraft({ ...draft, parts: draft.parts.filter((_, k) => k !== i) })}>×</button>}
            </div>
          ))}
          <div className="mixpart">
            <span className="swatch hatch" />
            <span>Backspace ⌫</span>
            <input type="range" min={0} max={10} step={0.1} value={100 * draft.edits} aria-label="Backspace share"
              onChange={(e) => setDraft({ ...draft, edits: +e.target.value / 100 })} />
            <span className="num">{fmt(100 * draft.edits, 1)}%</span>
          </div>
          {comp.nav > 0 && !edited && <div className="hint">Arrows {fmt(100 * comp.nav, 1)}% (modelled)</div>}
          <select value="" onChange={(e) => e.target.value && setDraft({ ...draft, parts: [...draft.parts, { corpus: e.target.value, weight: tot / Math.max(draft.parts.length, 1) }] })}>
            <option value="">Add a part…</option>
            {(["language", "component", "reference"] as const).map((k) => (
              <optgroup key={k} label={k[0].toUpperCase() + k.slice(1) + "s"}>
                {choices.filter((c) => c.kind === k).map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
              </optgroup>
            ))}
          </select>
          {edited && (
            <form className="namerow" onSubmit={(e) => { e.preventDefault(); if (name.trim() && !bundled) save(); }}>
              <input value={name} placeholder="New language name" onChange={(e) => setName(e.target.value)} />
              <button className="btn primary small" type="submit" disabled={busy || !name.trim() || bundled} title={bundled ? "That name is taken" : "Build this mix on the server; it can then be used for runs"}>Save</button>
              <button className="btn soft small" type="button" onClick={() => setDraft(saved)}>Reset</button>
            </form>
          )}
          <div className="mixnotes">
            {missing.length > 0 && <div><span className="muted">Not in this language:</span> <span className="mono">{missing.join(" ")}</span></div>}
            {zero.length > 0 && <div><span className="muted">In the charset, never typed:</span> <span className="mono">{zero.map(label).join(" ")}</span></div>}
            {!corpus.shift && <div className="muted">No case counts: capitals cost no Shift here.</div>}
          </div>
        </div>
        <div className="mixcharts">
          <MixBar names={names} shares={view.parts} />
          <div className="mixlegend">
            {names.map((n, i) => <span key={n}><span className="swatch" style={{ background: partColor(i) }} />{n}</span>)}
            {view.parts[names.length] > 0 && <span><span className="swatch hatch" />modelled (⌫, arrows)</span>}
          </div>
          {(["keys", "symbols"] as SymbolGroup[]).map((g) => (
            <SymbolBars key={g} syms={byShare(view.mono, g)} mono={view.mono} contrib={view.contrib} names={names}
              title={g === "keys" ? "Letters, space and keys" : "Punctuation and digits"} />
          ))}
        </div>
      </div>
    </div>
  );
}

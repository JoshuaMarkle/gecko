// Export (context/web.md "Export"): the last tab. Sidebar: name, image options and every file
// format (copy or download), and the 3D-print STL files. Main: the working model on its physical
// board, exactly as the SVG/PNG export draws it, or (3D, the default for boards with height or
// tilt) the printable case with switches and keycaps. Bottom: score, run facts and metrics when the layout was scored.

import { useEffect, useMemo, useState } from "react";
import type { Geometry } from "../../api";
import { boardSvg, ergogenYaml, kleJson, layerChars, layoutJson, layoutTextExport, layoutToml, slug, statsCsv, statsMarkdown, type ExportInfo, type ImageOptions, type KeyColor } from "../../export";
import { fmt, layerName, layerSlots, scoredMetrics, verdict } from "../../format";
import { Case3D } from "../../three/Case3D";
import { buildCase, exportLayout, wasmReady, type CaseModel, type CaseParams, type Switch } from "../../wasm";
import { Frame, Num, Sec, Seg, Toggle } from "../Form";

type Community = "oxeylyzer" | "genkey" | "cmini";
interface FileFormat { id: string; title: string; help: string; ext: string; mime: string; make: (e: ExportInfo) => string; stats?: boolean }

const IMAGE_SCALES: [string, string][] = [["1", "1×"], ["2", "2×"], ["4", "4×"]];

function save(blob: Blob, file: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url; a.download = file;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Rasterize a standalone SVG at `scale` × its size. */
async function toPng(svg: string, scale: number): Promise<Blob> {
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  try {
    const img = new Image();
    await new Promise<void>((ok, bad) => { img.onload = () => ok(); img.onerror = () => bad(new Error("the SVG did not load")); img.src = url; });
    const c = document.createElement("canvas");
    c.width = Math.round(img.width * scale); c.height = Math.round(img.height * scale);
    const ctx = c.getContext("2d");
    if (!ctx) throw new Error("no canvas");
    ctx.scale(scale, scale);
    ctx.drawImage(img, 0, 0);
    return await new Promise<Blob>((ok, bad) => c.toBlob((b) => (b ? ok(b) : bad(new Error("PNG encoding failed"))), "image/png"));
  } finally {
    URL.revokeObjectURL(url);
  }
}

/** Case parameters the sidebar edits (mm), with their help. */
const CASE_FIELDS: [keyof CaseParams, string, string][] = [
  ["plate", "Plate", "Plate thickness"],
  ["wall", "Wall", "Wall thickness at the floor"],
  ["clearance", "Clearance", "Floor to the lowest switch bottom (wiring room)"],
  ["bridge", "Bridge", "Largest gap between key edges a web spans; wider gaps split the plate"],
];
const STL_FILES: ["plate" | "case" | "bottom", string, string][] = [
  ["case", "Case", "Plate and walls in one part"],
  ["plate", "Plate", "Switch plate alone (sandwich or hand-wired build)"],
  ["bottom", "Bottom", "Flat bottom plate per half"],
];

/** Any key raised or tilted out of the plane: the board is 3D. */
const isFlat = (g: Geometry) => g.key.every((k) => k.pos[2] === 0 && k.rot[0] === 0 && k.rot[1] === 0);

const mm = (v: number) => v.toFixed(0);
const bytes = (n: number) => (n < 1e5 ? `${Math.ceil(n / 1e3)} KB` : `${(n / 1e6).toFixed(1)} MB`);

/** Community formats through gecko-core (`gecko layouts export`): 30 alpha keys only. */
function community(id: Community, title: string, help: string, ext: string): FileFormat {
  return { id, title, help, ext, mime: ext.endsWith("json") ? "application/json" : "text/plain", make: (e) => exportLayout(e.geometry, e.name, e.chars, id) };
}

export function ExportView({ info, label, onError }: { info: ExportInfo; label: string; onError: (msg: string) => void }) {
  const [name, setName] = useState(label);
  useEffect(() => setName(label), [label]);
  const [color, setColor] = useState<KeyColor>("plain");
  const [layers, setLayers] = useState(true);
  const [title, setTitle] = useState(true);
  const [scale, setScale] = useState("2");
  const [copied, setCopied] = useState<string | null>(null);
  const [wasm, setWasm] = useState(false);
  useEffect(() => { wasmReady().then(() => setWasm(true)).catch(() => {}); }, []);
  const [view3d, setView3d] = useState(() => !isFlat(info.geometry));
  useEffect(() => setView3d(!isFlat(info.geometry)), [info.geometry.id]); // eslint-disable-line react-hooks/exhaustive-deps
  const [caps, setCaps] = useState(true);
  const [layer, setLayer] = useState(0);
  const [showBottom, setShowBottom] = useState(true);
  const [sw, setSw] = useState<Switch>("mx");
  const [over, setOver] = useState<Partial<CaseParams>>({});
  const build = useMemo((): { model?: CaseModel; err?: string } => {
    if (!wasm) return {};
    try { return { model: buildCase(info.geometry, sw, over) }; } catch (err) { return { err: (err as Error).message }; }
  }, [wasm, info.geometry, sw, over]);

  const e: ExportInfo = useMemo(() => ({ ...info, name: name.trim() || label || "layout" }), [info, name, label]);
  const g: Geometry = e.geometry;
  const perLayer = useMemo(() => layerChars(g, e.chars), [g, e.chars]);
  const layered = perLayer.length > 1;
  const shownLayer = Math.min(layer, perLayer.length - 1);
  const names = useMemo(() => { const s = layerSlots(g, e.chars); return s.map((_, l) => layerName(e.chars, s, l)); }, [g, e.chars]);
  const opts: ImageOptions = { color: color === "heat" && !e.a ? "plain" : color, layers: layered && layers, title };
  const svg = useMemo(() => boardSvg(e, opts), [e, opts.color, opts.layers, opts.title]); // eslint-disable-line react-hooks/exhaustive-deps
  const preview = useMemo(() => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`, [svg]);
  const base = `${slug(e.name)}-${slug(g.id)}`;

  const formats: [string, FileFormat[]][] = [
    ["Files", [
      { id: "toml", title: "GECKO layout", help: "Layout file (data/layouts format): `gecko score file.toml`, `--from file.toml`", ext: "toml", mime: "application/toml", make: layoutToml },
      { id: "txt", title: "Text", help: "Rows per layer; a blank line between layers", ext: "txt", mime: "text/plain", make: layoutTextExport },
      { id: "json", title: "JSON", help: "Layout, layers, score, every metric and finger load", ext: "json", mime: "application/json", make: layoutJson },
    ]],
    ["Other tools", [
      { id: "kle", title: "KLE", help: "keyboard-layout-editor.com: board, legends (layers as extra legends) and the image's key colors", ext: "kle.json", mime: "application/json", make: (x) => kleJson(x, opts.color) },
      { id: "ergogen", title: "Ergogen", help: "ergogen.xyz config: every key's position and rotation in mm (one zone per key) and a switch plate outline, to build a PCB or case from", ext: "ergogen.yaml", mime: "application/yaml", make: ergogenYaml },
      community("oxeylyzer", "Oxeylyzer", "Oxeylyzer .dof: the 30 alpha keys", "dof.json"),
      community("genkey", "Genkey", "Genkey layout file: the 30 alpha keys", "genkey.txt"),
      community("cmini", "cmini", "cmini JSON: the 30 alpha keys", "cmini.json"),
    ]],
    ["Stats", [
      { id: "csv", title: "Metrics CSV", help: "Every metric: value, weight, contribution (and the reference's value)", ext: "csv", mime: "text/csv", make: statsCsv, stats: true },
      { id: "md", title: "Markdown", help: "A summary for a README, forum post or issue: layout and weighted metrics", ext: "md", mime: "text/markdown", make: statsMarkdown },
    ]],
  ];

  /** Why a format cannot be made right now, or null. */
  const blocked = (f: FileFormat): string | null => {
    if (f.stats && !e.a) return "Not scored yet";
    if (["oxeylyzer", "genkey", "cmini"].includes(f.id)) {
      if (!wasm) return "Loading…";
      try { f.make(e); } catch (err) { return `Unavailable: ${(err as Error).message}`; }
    }
    return null;
  };

  const run = (f: FileFormat, how: "copy" | "save") => {
    try {
      const text = f.make(e);
      if (how === "save") save(new Blob([text], { type: f.mime }), `${base}.${f.ext}`);
      else navigator.clipboard.writeText(text).then(() => { setCopied(f.id); setTimeout(() => setCopied((c) => (c === f.id ? null : c)), 1500); }, () => onError("The clipboard is not available here; use Download."));
    } catch (err) {
      onError(`${f.title} export failed: ${(err as Error).message}`);
    }
  };

  const savePng = () => toPng(svg, Number(scale)).then((b) => save(b, `${base}.png`)).catch((err) => onError(`PNG export failed: ${(err as Error).message}`));
  const saveSvg = () => save(new Blob([svg], { type: "image/svg+xml" }), `${base}.svg`);
  const model = build.model;
  const rep = model?.report;
  const sizes = rep?.pieces.map((p) => `${mm(p.size[0])} × ${mm(p.size[1])} × ${mm(p.size[2])}`).join(" · ");

  const a = e.a, ref = e.ref;
  const delta = a && ref ? a.breakdown.score - ref.a.breakdown.score : null;
  const metrics = a ? scoredMetrics(a.breakdown.metrics) : [];
  const maxLoad = a ? Math.max(...a.finger_load, 1e-9) : 1;

  return (
    <Frame
      title="Export"
      control={<span className="hint">{g.name}</span>}
      side={<>
        <Sec id="export.name" title="Layout">
          <div className="row"><label title="Name written into the files and the image title">Name</label>
            <input type="text" value={name} onChange={(ev) => setName(ev.target.value)} placeholder={label} /><span /></div>
        </Sec>
        <Sec id="export.image" title="Image" note="PNG · SVG">
          <div className="row"><label title="Key fill: plain, finger colors, or keystroke heat (needs stats)">Key color</label>
            <Seg value={opts.color} onChange={setColor} options={[["plain", "Plain"], ["fingers", "Fingers"], ...(a ? [["heat", "Heat"]] as [KeyColor, string][] : [])]} /><span /></div>
          {layered && <Toggle label="All layers" help="One board per layer, stacked; off: the base layer only" value={layers} onChange={setLayers} />}
          <Toggle label="Title and score" help="The name, board, language, profile and score above the board" value={title} onChange={setTitle} />
          <div className="row"><label title="PNG size relative to the SVG">PNG scale</label><Seg value={scale} onChange={setScale} options={IMAGE_SCALES} /><span /></div>
          <div className="btns exportimg">
            <button className="btn primary" onClick={savePng}>Download PNG</button>
            <button className="btn outline" onClick={saveSvg}>Download SVG</button>
          </div>
        </Sec>
        <Sec id="export.stl" title="3D print" note="STL">
          <div className="row"><label title="Switch cutout and keycap height">Switch</label>
            <Seg value={sw} onChange={setSw} options={[["mx", "MX"], ["choc", "Choc"]]} /><span /></div>
          {CASE_FIELDS.map(([k, label, help]) => (
            <Num key={k} label={label} help={help} unit="mm" step={0.5} min={0} value={(over[k] as number | undefined) ?? (model?.params[k] as number | undefined) ?? NaN}
              onChange={(v) => setOver((o) => ({ ...o, [k]: v }))} />
          ))}
          {build.err && <div className="hint err">{build.err}</div>}
          {STL_FILES.map(([part, title, help]) => (
            <div key={part} className="fmtrow" title={help}>
              <div><b>{title}</b><span className="hint">{model ? bytes(model.stl[part].length) : wasm ? "Unavailable" : "Loading…"}</span></div>
              <div className="btns">
                <button className="btn outline small" disabled={!model} onClick={() => model && save(new Blob([model.stl[part] as BlobPart], { type: "model/stl" }), `${slug(g.id)}-${sw}-${part}.stl`)}>Download</button>
              </div>
            </div>
          ))}
          {rep && (
            <div className="casenotes">
              <span className="hint">{rep.keys} switches · {rep.pieces.length === 1 ? "1 piece" : `${rep.pieces.length} pieces`} · {sizes} mm</span>
              {rep.dropped > 0 && <span className="hint">{rep.dropped} connector{rep.dropped > 1 ? "s" : ""} left out (would block a switch)</span>}
              {rep.blocked.length > 0 && <span className="hint err">Switch holes blocked: {rep.blocked.length} keys</span>}
              {rep.cap_hits.length > 0 && <span className="hint err">Keycaps hit the case when pressed: {rep.cap_hits.length} keys (red)</span>}
            </div>
          )}
        </Sec>
        {formats.map(([group, fs]) => (
          <Sec key={group} id={`export.${group}`} title={group}>
            {fs.map((f) => {
              const why = blocked(f);
              return (
                <div key={f.id} className="fmtrow" title={why ?? f.help}>
                  <div><b>{f.title}</b><span className="hint">{why ?? `.${f.ext}`}</span></div>
                  <div className="btns">
                    <button className="btn soft small" disabled={!!why} onClick={() => run(f, "copy")}>{copied === f.id ? "Copied" : "Copy"}</button>
                    <button className="btn outline small" disabled={!!why} onClick={() => run(f, "save")}>Download</button>
                  </div>
                </div>
              );
            })}
          </Sec>
        ))}
      </>}
      main={
        <div className="viewport exportview">
          {!view3d ? <img src={preview} alt={`${e.name} on ${g.name}`} />
            : model ? <Case3D geometry={g} chars={perLayer[shownLayer]?.join("") ?? e.chars} model={model} caps={caps} bottom={showBottom} />
            : <div className="empty">{build.err ? `No case: ${build.err}` : "Building the case…"}</div>}
          <div className="viewlabel"><span>{e.name}</span>{a && <span className="muted">{fmt(a.breakdown.score, 3)}</span>}
            {layered && <span className="muted">{view3d ? names[shownLayer] : layers ? "all layers" : "base layer"}</span>}</div>
          <div className="viewctl">
            <div className="seg inline"><button className={view3d ? "on" : ""} onClick={() => setView3d(!view3d)} title="3D: the printable case (STL); off: the image export">3D</button></div>
            {view3d && layered && caps && <Seg inline value={`${shownLayer}`} onChange={(v) => setLayer(+v)} options={names.map((n, l) => [`${l}`, n] as [string, string])} />}
            {!view3d && layered && <div className="seg inline"><button className={layers ? "on" : ""} onClick={() => setLayers(!layers)} title="One board per layer, stacked; off: the base layer only">All layers</button></div>}
            {view3d && <div className="seg inline"><button className={caps ? "on" : ""} onClick={() => setCaps(!caps)} title="Switches and keycaps on the plate">Keycaps</button></div>}
            {view3d && <div className="seg inline"><button className={showBottom ? "on" : ""} onClick={() => setShowBottom(!showBottom)} title="Bottom plate">Bottom</button></div>}
          </div>
          {view3d && rep && <div className="viewhint">{sw === "mx" ? "MX" : "Choc"} case · {sizes} mm{rep.cap_hits.length > 0 ? " · red keycaps hit the case when pressed" : ""}</div>}
        </div>
      }
      bottom={
        !a ? <div className="empty">No stats: this layout has not been scored (custom boards need the WASM scorer). Files without stats still export.</div> : (
          <div className="exportgrid">
            <div>
              <h4>Score <span className="hint">lower is better</span></h4>
              <div className="score">
                <b>{fmt(a.breakdown.score, 3)}</b>
                {delta !== null && <span className={`badge ${delta < -1e-9 ? "success" : delta > 1e-9 ? "error" : ""}`}>{delta <= 0 ? "" : "+"}{fmt(delta, 3)} vs {ref!.name}</span>}
              </div>
              <div className="kv"><span>Board</span><span>{g.name}</span></div>
              {e.corpus && <div className="kv"><span>Language</span><span>{e.corpus}</span></div>}
              {e.profile && <div className="kv"><span>Profile</span><span>{e.profile}</span></div>}
              {e.run && <div className="kv"><span>Run</span><span className="mono">{e.run.algo} · {e.run.id}</span></div>}
              {e.run?.holdout && <div className="kv"><span>Holdout</span><span>{e.run.holdout.corpus} · keeps {fmt(100 * e.run.holdout.retained, 1)}% of the gain</span></div>}
            </div>
            <div>
              <h4>Weighted metrics <span className="hint">value{ref ? ` · ${ref.name}` : ""} · contribution</span></h4>
              <table className="tbl">
                <tbody>
                  {metrics.map((m) => {
                    const r = ref?.a.breakdown.metrics.find((x) => x.id === m.id);
                    const v = r ? verdict(m.dir, m.value, r.value) : "same";
                    const u = m.unit === "%" ? "%" : "";
                    return <tr key={m.id} title={m.id}><td>{m.name}</td><td>{fmt(m.value)}{u}</td><td className="muted">{r ? `${fmt(r.value)}${u}` : ""}</td><td>{fmt(m.weighted, 3)}</td><td>{v !== "same" && <span className={`dot ${v}`} />}</td></tr>;
                  })}
                </tbody>
              </table>
            </div>
            <div>
              <h4>Finger load <span className="hint">% of keystrokes</span></h4>
              {a.fingers.map((f, i) => a.finger_load[i] > 0 && (
                <div key={f} className="kv"><span>{f}</span><span className="bar"><i style={{ width: `${(100 * a.finger_load[i]) / maxLoad}%` }} /><em>{fmt(a.finger_load[i], 1)}</em></span></div>
              ))}
            </div>
          </div>
        )
      }
    />
  );
}

// Language tab (web-screens.md "Language"): languages with their sources | text viewer |
// language statistics. Summarize lives in the navbar (App).

import { useEffect, useRef, useState } from "react";
import { INK } from "../../colors";
import { fmt, fmtCount } from "../../format";
import { useLanguageStats, type Language, type Languages } from "../../languages";
import { useSize } from "../Charts";
import { Frame } from "../Form";

interface Props {
  langs: Languages;
  /** Corpus used for scoring and runs (bundled languages only). */
  corpusId: string;
  setCorpusId: (id: string) => void;
}

/** Cap on characters shown in the text viewer (whole books are several MB). */
const VIEW_CAP = 400_000;

const fmtBytes = (n: number) => (n >= 1e6 ? `${(n / 1e6).toFixed(1)} MB` : n >= 1e3 ? `${(n / 1e3).toFixed(0)} kB` : `${n} B`);

function StatusLabel({ s }: { s: Language["status"] }) {
  return <span className={`status ${s.toLowerCase()}`}>{s}</span>;
}

function KeyFreq({ mono }: { mono: [string, number][] }) {
  const [box, { w, h }] = useSize<HTMLDivElement>();
  const items = mono.filter(([, v]) => v > 0);
  const max = Math.max(...items.map(([, v]) => v), 1e-9);
  const m = { l: 4, r: 4, t: 8, b: 18 };
  const bw = (w - m.l - m.r) / Math.max(items.length, 1);
  return (
    <div ref={box} className="chartfill">
      <svg width={w} height={h}>
        {items.map(([c, v], i) => {
          const bh = (v / max) * (h - m.t - m.b);
          return (
            <g key={c}>
              <title>{`${c} ${fmt(v, 2)}%`}</title>
              <rect x={m.l + i * bw + 1} y={h - m.b - bh} width={Math.max(1, bw - 2)} height={bh} fill={INK} rx={1} />
              {bw >= 8 && <text x={m.l + i * bw + bw / 2} y={h - 5} textAnchor="middle" style={{ fontFamily: "var(--mono)" }}>{c === " " ? "␣" : c}</text>}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function Grams({ title, rows }: { title: string; rows: [string, number][] }) {
  const max = Math.max(...rows.map(([, v]) => v), 1e-9);
  return (
    <div>
      <h4>{title}</h4>
      <table className="tbl">
        <tbody>
          {rows.slice(0, 12).map(([g, v]) => (
            <tr key={g}><td className="mono">{g.replace(/ /g, "␣")}</td><td>{fmt(v, 2)}%</td><td style={{ width: "45%" }}><span className="inbar" style={{ width: `${(100 * v) / max}%` }} /></td></tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Stats({ lang }: { lang: Language }) {
  const { stats, error } = useLanguageStats(lang);
  if (lang.status === "Unsummarized") {
    return <div className="empty">{lang.sources?.length ? "Not summarized yet. Press Summarize in the navbar to build its statistics." : "No sources yet. Upload text files, then Summarize."}</div>;
  }
  if (error) return <div className="pad"><div className="notice error">{error}</div></div>;
  if (!stats) return <div className="empty">Loading statistics…</div>;
  return (
    <div className="langstats">
      <div className="langfacts">
        <div className="kv"><span>Characters</span><span>{fmtCount(stats.chars)}</span></div>
        <div className="kv"><span>Words</span><span>{lang.words !== null ? fmtCount(lang.words) : "–"}</span></div>
        <div className="kv"><span>Sources</span><span>{lang.sources ? lang.sources.length : "–"}</span></div>
        <div className="kv"><span>Charset</span><span className="mono" title={stats.charset}>{stats.charset.length} symbols</span></div>
        {lang.status === "Outdated" && <div className="notice warn">Sources changed since the last summary. Summarize again to update.</div>}
      </div>
      <div className="langfreq"><h4>Key frequencies <span className="hint">% of keystrokes</span></h4><KeyFreq mono={stats.mono} /></div>
      <Grams title="Top bigrams" rows={stats.bigrams} />
      <Grams title="Top trigrams" rows={stats.trigrams} />
      <Grams title="Top skipgrams" rows={stats.skipgrams} />
    </div>
  );
}

export function LanguageView({ langs, corpusId, setCorpusId }: Props) {
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const [src, setSrc] = useState<{ lang: string; name: string } | null>(null);
  const [text, setText] = useState<{ body: string; total: number } | null>(null);
  const [textErr, setTextErr] = useState<string | null>(null);
  const [naming, setNaming] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const sel = langs.selected;

  useEffect(() => {
    let alive = true;
    setText(null);
    setTextErr(null);
    if (!src) return;
    langs.sourceText(src.lang, src.name)
      .then((t) => alive && setText({ body: t.slice(0, VIEW_CAP), total: t.length }))
      .catch((e) => alive && setTextErr((e as Error).message));
    return () => { alive = false; };
  }, [src]); // eslint-disable-line react-hooks/exhaustive-deps

  const toggle = (l: Language) => {
    const o = !open[l.id];
    setOpen({ ...open, [l.id]: o });
    if (o) langs.loadSources(l.id);
  };
  const choose = (l: Language) => {
    langs.select(l.id);
    if (!l.local) setCorpusId(l.id);
  };

  return (
    <Frame
      title={<>Languages <span className="muted">· {langs.languages.length}</span></>}
      side={
        <div className="langlist">
          {langs.languages.map((l) => (
            <div key={l.id} className={`langrow ${sel?.id === l.id ? "sel" : ""}`}>
              <div className="langhead">
                <button className={`chev ${open[l.id] ? "open" : ""}`} onClick={() => toggle(l)} aria-label={open[l.id] ? "Collapse" : "Expand"} />
                <button className="langname" onClick={() => choose(l)} title={l.local ? "Created in this browser" : "Bundled corpus"}>
                  {l.name}
                  {!l.local && l.id === corpusId && <span className="hint"> · in use</span>}
                  {l.local && <span className="hint"> · local</span>}
                </button>
                <StatusLabel s={l.status} />
              </div>
              {open[l.id] && (
                <div className="sources">
                  {l.sources === null && <div className="hint">Loading…</div>}
                  {l.sources?.length === 0 && <div className="hint">{l.local ? "No sources. Upload text files below." : "No source texts on this device."}</div>}
                  {l.sources?.map((s) => (
                    <button key={s.name} className={`source ${src?.lang === l.id && src.name === s.name ? "sel" : ""}`} onClick={() => { choose(l); setSrc({ lang: l.id, name: s.name }); }}>
                      <span className="mono">{s.name}</span><span className="muted">{fmtBytes(s.bytes)}</span>
                    </button>
                  ))}
                  {l.local && <button className="btn soft small" onClick={() => langs.remove(l.id)}>Delete language</button>}
                </div>
              )}
            </div>
          ))}
        </div>
      }
      footer={
        <>
          {naming !== null ? (
            <form className="namerow" onSubmit={(e) => { e.preventDefault(); if (naming.trim()) langs.create(naming.trim()); setNaming(null); }}>
              <input autoFocus value={naming} placeholder="Language name" onChange={(e) => setNaming(e.target.value)} onKeyDown={(e) => e.key === "Escape" && setNaming(null)} />
              <button className="btn primary small" type="submit">Create</button>
            </form>
          ) : (
            <div className="btns">
              <button className="btn outline small" onClick={() => setNaming("")}>New language</button>
              <button className="btn outline small" disabled={!sel?.local} title={sel?.local ? `Add text files to ${sel.name}` : "Bundled languages are read-only; create a new language to add sources."}
                onClick={() => fileRef.current?.click()}>Upload sources</button>
            </div>
          )}
          <input ref={fileRef} type="file" multiple accept=".txt,text/plain" hidden onChange={(e) => {
            const files = [...(e.target.files ?? [])];
            if (sel?.local && files.length) langs.upload(sel.id, files);
            e.target.value = "";
          }} />
        </>
      }
      main={
        !src ? <div className="empty faint">No text selected</div> : (
          <div className="textview">
            <div className="textview-head"><b className="mono">{src.name}</b>{text && <span className="hint">{fmtCount(text.total)} characters{text.total > VIEW_CAP ? ` · showing the first ${fmtCount(VIEW_CAP)}` : ""}</span>}</div>
            {textErr ? <div className="pad"><div className="notice error">{textErr}</div></div> : <pre>{text ? text.body : "Loading…"}</pre>}
          </div>
        )
      }
      bottom={sel ? <Stats lang={sel} /> : <div className="empty">No language.</div>}
    />
  );
}

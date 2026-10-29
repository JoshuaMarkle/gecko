// Scene 7: through the board into the data behind it. A tunnel of the product's charts
// (bigram heat, finger load, the comfort table, the hand profile's weights, per-key effort, the
// search's worker traces, the explore atlas, MAP-Elites, letter frequency, top bigrams) in mirrored
// pairs facing the camera, one pair every station; the camera accelerates down it and fades out.

import { AbsoluteFill } from "remotion";
import { bigramMatrix, data } from "../lib/data";
import { EASE, progress } from "../lib/motion";
import { C, fingerColor } from "../lib/tokens";
import { Board2D } from "../ui/Board2D";
import { Bars, EliteGrid, Heatmap, LineChart, Scatter } from "../ui/charts";
import { MonoList } from "../ui/panels";
import { Tunnel as TunnelView } from "../ui/space";
import { useLayout } from "./common";

const L = data.layout;
const A = data.analyze.analysis as unknown as { finger_load: number[]; fingers: string[]; heat: number[] };
const big = bigramMatrix();
const letters = (data.language.corpus.symbols_per_mille as { symbol: string; share: number }[]).filter((s) => /^[a-z]$/.test(s.symbol)).slice(0, 26);
const atlas = data.search.atlas.points.map(([x, y, s]) => ({ x, y, s }));
const heatMax = Math.max(...A.heat);
const weights = data.profileToml.split("\n").map((l) => l.match(/^([a-z_]+)\s*=\s*([0-9.]+)/)).filter((m): m is RegExpMatchArray => !!m).map((m) => [m[1], (+m[2]).toFixed(3)] as [string, string]);
const engram = data.engram.filter((l) => !l.startsWith("#")).slice(0, 40).map((l) => { const [a, ...r] = l.split(","); return [a, r.join("  ")] as [string, string]; });
const trigrams = (data.analyze.analysis as unknown as { bigrams: { ngram: string; freq: number }[] }).bigrams.slice(0, 40).map((b) => [b.ngram, `${b.freq.toFixed(2)}%`] as [string, string]);

const PW = 600, PH = 600, CW = 560, CH = 420;
function panels(frame: number) {
  return [
    <Heatmap key="h" w={CH} h={CH} matrix={big.m} />,
    <Bars key="b" w={CW} h={CH} values={A.finger_load} labels={A.fingers} colors={A.fingers.map(fingerColor)} />,
    <div key="e" style={{ width: CW }}><MonoList height={CH} size={28} lines={engram} scroll={frame / 6} /></div>,
    <Board2D key="k" u={CW / 17.5} chars={L.final} fill="heat" heat={A.heat.map((h) => h / heatMax)} />,
    <LineChart key="c" w={CW} h={CH} series={L.workers[0].map((_, j) => ({ points: L.workers.map((w, i) => [i, w[j]] as [number, number]), width: 2, color: j ? "#5c9dff" : C.ink }))} yDomain={[9, 60]} />,
    <Scatter key="s" w={CW} h={CH} points={atlas} best={data.search.atlas.best} bg={false} />,
    <div key="w" style={{ width: CW }}><MonoList height={CH} size={28} lines={weights} scroll={frame / 8} /></div>,
    <EliteGrid key="q" w={CH} h={CH} cells={data.search.qd.grid} />,
    <Bars key="l" w={CW} h={CH} values={letters.map((l) => l.share)} labels={letters.map((l) => l.symbol)} colors={letters.map((_, i) => (i < 5 ? C.accent : C.ink))} />,
    <div key="t" style={{ width: CW }}><MonoList height={CH} size={28} lines={trigrams} scroll={frame / 7} /></div>,
  ];
}

export function Tunnel() {
  const { frame, dur } = useLayout();
  // Ease into speed and keep accelerating; the frame fades to white at the end (no page waiting).
  const p = 0.92 * Math.pow(frame / dur, 1.6);
  const fadeIn = progress(frame, 6, 14, EASE.linear);
  const fadeOut = progress(frame, dur - 16, 14, EASE.in);
  const ps = panels(frame);
  return (
    <AbsoluteFill style={{ background: C.canvas, opacity: fadeIn * (1 - fadeOut) }}>
      <TunnelView panels={[...ps, ...ps.slice(0, 6)]} progress={p} panelW={PW} panelH={PH} />
    </AbsoluteFill>
  );
}

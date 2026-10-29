// Preview: every 2D component with real data, one page per second-and-a-half, building in.

import { AbsoluteFill, Sequence } from "remotion";
import { bigramMatrix, data, fmt } from "../lib/data";
import { EASE, interp, progress, sec } from "../lib/motion";
import { C, fingerColor, TYPE } from "../lib/tokens";
import { Stage, useLayout } from "../scenes/common";
import { Board2D } from "../ui/Board2D";
import { Bars, Card, EliteGrid, Heatmap, LineChart, RangeBars, Scatter, StackedBar } from "../ui/charts";
import { MetricCounter } from "../ui/numbers";
import { DataTable, EndCard, FileCard, MonoList, Page, SegmentedTabs, StatusBadge, TerminalWindow } from "../ui/panels";
import { LayoutPage } from "../ui/LayoutPage";
import { Tunnel } from "../ui/space";
import { Eyebrow, GlitchText, KineticHeadline } from "../ui/type";

const PAGE = sec(2.5);
const L = data.layout, B = data.boardOpt, A = data.analyze.analysis as unknown as { finger_load: number[]; fingers: string[]; heat: number[] };
const big = bigramMatrix();
const RANGE_LABEL: Record<string, string> = { "split.tent": "Tent", "fingers.row_curve": "Row curve", "split.angle": "Split angle", "thumbs.tilt": "Thumb tilt" };
const ranges = B.ranges.map((r) => ({ label: RANGE_LABEL[r.name] ?? r.name, lo: r.lo, hi: r.hi, start: r.start, value: r.value }));
const letters = (data.language.corpus.symbols_per_mille as { symbol: string; share: number }[]).filter((s) => /^[a-z]$/.test(s.symbol)).slice(0, 26);
const recipe = (data.language.corpus.recipe as { part: { corpus: string; weight: number }[] }).part.map((p) => ({ label: p.corpus.replace("prose-", ""), weight: p.weight }));
const atlas = data.search.atlas.points.map(([x, y, s]) => ({ x, y, s }));
const heatMax = Math.max(...A.heat);

function Charts() {
  const { frame } = useLayout();
  const r = progress(frame, 0, sec(1.6), EASE.inOut);
  return (
    <AbsoluteFill style={{ padding: 80, display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gridTemplateRows: "1fr 1fr", gap: 32 }}>
      <Card title="Board search" note={`${B.snapshots.length} generations`}>
        <LineChart w={500} h={330} reveal={r} series={[{ points: B.convergence, step: true }]} highlight={B.convergence.at(-1) as [number, number]}
          threshold={{ y: B.start, label: "start" }} yFormat={(v) => v.toFixed(0)} xLabel="generation" />
      </Card>
      <Card title="Finger load" note="% of presses">
        <Bars w={500} h={330} reveal={r} values={A.finger_load} labels={A.fingers} colors={A.fingers.map(fingerColor)} format={(v) => v.toFixed(0)} />
      </Card>
      <Card title="Letter bigrams" note={data.corpus}>
        <Heatmap w={330} h={330} reveal={r} matrix={big.m} />
      </Card>
      <Card title="Letters" note={data.corpus}>
        <Bars w={500} h={300} reveal={r} values={letters.map((l) => l.share)} labels={letters.map((l) => l.symbol)} />
        <div style={{ height: 8 }} />
      </Card>
      <Card title="Explore atlas" note={`${atlas.length} local optima`}>
        <Scatter w={500} h={330} reveal={r} points={atlas} best={data.search.atlas.best} />
      </Card>
      <Card title="MAP-Elites" note="roll ratio × index load">
        <EliteGrid w={330} h={330} reveal={r} cells={data.search.qd.grid} />
      </Card>
    </AbsoluteFill>
  );
}

function Panels() {
  const { frame } = useLayout();
  const r = progress(frame, 0, sec(1.4), EASE.inOut);
  const rows = data.board.keys.slice(0, 6).map((k, i) => [L.start[i] === "⇥" ? "Tab" : L.start[i].toUpperCase(), k.pose[0].toFixed(2), k.pose[1].toFixed(2), "0°", k.finger, k.effort.toFixed(1)]);
  return (
    <AbsoluteFill style={{ padding: 80, display: "flex", flexDirection: "row", gap: 48 }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 40 }}>
        <DataTable width={760} size={22} reveal={r} columns={[{ label: "key", align: "left" }, { label: "x" }, { label: "y" }, { label: "rot" }, { label: "finger", align: "left" }, { label: "effort" }]} rows={rows} />
        <Card title="Board" note="free parameters" width={760}>
          <RangeBars w={700} rows={ranges} count={progress(frame, sec(0.3), sec(1.6), EASE.inOut)} />
        </Card>
        <div style={{ display: "flex", gap: 24, alignItems: "center" }}>
          <SegmentedTabs options={["Layout", "Board"]} active={interp(frame, [[sec(0.6), 0], [sec(1.2), 1]])} />
          <StatusBadge kind="success" text="No collisions" start={sec(0.8)} />
          <StatusBadge kind="best" text="Best" start={sec(1)} />
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 40 }}>
        <FileCard name="case.stl" size={`${Math.round(data.stl.case.bytes / 1024)} KB`} dims={data.stl.build.files[1].size_mm.map((v) => Math.round(v)).join(" × ") + " mm"} reveal={r} width={760} />
        <TerminalWindow width={760} reveal={r} command="gecko perf --json" lines={[
          `full    ${fmt.int(data.perf.full_per_sec)} / s`, `delta   ${fmt.int(data.perf.delta_per_sec)} / s`,
          `build   ${fmt.int(data.perf.build_per_sec)} / s`, `rebuild ${fmt.int(data.perf.rebuild_per_sec)} / s`]} />
        <Card title="Weights" note="profile hand" width={760} height={300}>
          <MonoList height={220} scroll={interp(frame, [[0, 0], [PAGE, 6]], EASE.linear)} lines={Object.entries({ sfb: 2.867, pinky_ring: 1.176, hand_switch: 1.003, wrist: 1, thumb_arc: 1, outroll: 0.7151, bad_redirect: 0.5674, stretch: 0.5046, reach: 0.4272, hand_balance: 0.4181 }).map(([k, v]) => [k, v.toFixed(3)])} />
        </Card>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 24, justifyContent: "center" }}>
        <MetricCounter from={L.qwerty.score} to={L.best.score} count={progress(frame, 0, sec(1.6), EASE.inOut)} label="layout score" accent={frame > sec(1.7)} size={TYPE.display * 0.8} />
        <StackedBar w={300} parts={recipe} reveal={r} />
      </div>
    </AbsoluteFill>
  );
}

function App() {
  const { frame } = useLayout();
  return (
    <AbsoluteFill>
      <LayoutPage t={progress(frame, sec(0.2), sec(1.8), EASE.inOut)} reveal={progress(frame, 0, sec(1), EASE.out)} />
    </AbsoluteFill>
  );
}

function Type() {
  const { frame } = useLayout();
  return (
    <AbsoluteFill style={{ background: C.soft, padding: 80, display: "flex", flexDirection: "row", gap: 80, alignItems: "center" }}>
      <Page width={760} height={900} margin={80}>
        <div style={{ fontSize: 30, lineHeight: 1.55 }}>
          <GlitchText text={data.language.opening.slice(0, 330)} seed={3} glitch={interp(frame, [[0, 0], [sec(0.6), 0], [sec(1.6), 1]])}
            lift={interp(frame, [[sec(1.0), 0], [sec(2.4), 1]])} size={30} />
        </div>
      </Page>
      <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
        <Eyebrow text="Language." index="07" />
        <KineticHeadline text="Learned from what you type." start={4} width={900} />
        <div style={{ height: 60 }} />
        <EndCard start={sec(0.8)} />
      </div>
    </AbsoluteFill>
  );
}

function TunnelPage() {
  const { frame } = useLayout();
  const panels = [
    <Heatmap key="h" w={1050} h={640} matrix={big.m} />,
    <Bars key="b" w={1050} h={640} values={A.finger_load} labels={A.fingers} colors={A.fingers.map(fingerColor)} />,
    <MonoList key="e" height={640} size={30} lines={data.engram.slice(1, 22).map((l) => { const [a, ...r] = l.split(","); return [a, r.join("  ")]; })} />,
    <Board2D key="k" u={68} chars={L.final} fill="heat" heat={A.heat.map((h) => h / heatMax)} />,
    <Scatter key="s" w={1050} h={640} points={atlas} best={data.search.atlas.best} />,
    <LineChart key="c" w={1050} h={640} series={L.workers[0].map((_, j) => ({ points: L.workers.map((w, i) => [i, w[j]] as [number, number]), width: 2 }))} yDomain={[9, 60]} />,
    <EliteGrid key="q" w={1050} h={640} cells={data.search.qd.grid} />,
    <Bars key="l" w={1050} h={640} values={letters.map((l) => l.share)} labels={letters.map((l) => l.symbol)} />,
  ];
  return <AbsoluteFill><Tunnel panels={[...panels, ...panels]} progress={interp(frame, [[0, 0.0], [PAGE, 0.7]], EASE.in)} /></AbsoluteFill>;
}

export function Gallery() {
  const pages = [Charts, Panels, App, Type, TunnelPage];
  return (
    <Stage>
      {pages.map((P, i) => <Sequence key={i} from={i * PAGE} durationInFrames={PAGE}><P /></Sequence>)}
    </Stage>
  );
}

export const GALLERY_FRAMES = PAGE * 5;

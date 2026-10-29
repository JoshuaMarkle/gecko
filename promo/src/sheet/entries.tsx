// The component sheet's entries: each 2D component as it stands once built in (real data), and the
// 3D models to orbit by hand.

import { OrbitControls } from "@react-three/drei";
import { useThree } from "@react-three/fiber";
import { useLayoutEffect, type ReactNode } from "react";
import { AbsoluteFill } from "remotion";
import { bigramMatrix, data, fmt } from "../lib/data";
import { C, fingerColor, TYPE } from "../lib/tokens";
import { boardPosesAt, Corne3D } from "../three/Corne3D";
import { Hands3D } from "../three/Hands3D";
import { typing } from "../three/hands";
import { NiceNano, ResetButton, TRRS } from "../three/NiceNano";
import { Scene3D } from "../three/Scene3D";
import { STL_KEY_LIFT, STLModel } from "../three/STLModel";
import { Key } from "../three/Switch";
import { Board2D } from "../ui/Board2D";
import { Bars, Card, EliteGrid, Heatmap, LineChart, RangeBars, Scatter, StackedBar } from "../ui/charts";
import { DocChrome } from "../ui/Document";
import { LayoutPage } from "../ui/LayoutPage";
import { MetricCounter, StatRing } from "../ui/numbers";
import { DataTable, EndCard, FileCard, FlowChart, MonoList, Navbar, SegmentedTabs, StatusBadge, TerminalWindow } from "../ui/panels";
import { Eyebrow, KineticHeadline } from "../ui/type";

export interface Item2D {
  title: string;
  /** Source file under promo/src. */
  file: string;
  /** Frame size (px); the card scales it to fit. */
  w: number; h: number;
  wide?: boolean;
  el: () => ReactNode;
}

export interface Model3D {
  title: string;
  file: string;
  /** Start view (mm); orbit from there. */
  cam: [number, number, number];
  target: [number, number, number];
  floor?: boolean;
  el: () => ReactNode;
}

// ---- data, as in previews/Gallery.tsx ----
const L = data.layout, B = data.boardOpt;
const A = data.analyze.analysis as unknown as { finger_load: number[]; fingers: string[]; heat: number[] };
const big = bigramMatrix();
const heatMax = Math.max(...A.heat);
const heatN = A.heat.map((h) => h / heatMax);
const RANGE_LABEL: Record<string, string> = { "split.tent": "Tent", "fingers.row_curve": "Row curve", "split.angle": "Split angle", "thumbs.tilt": "Thumb tilt" };
const ranges = B.ranges.map((r) => ({ label: RANGE_LABEL[r.name] ?? r.name, lo: r.lo, hi: r.hi, start: r.start, value: r.value }));
const letters = (data.language.corpus.symbols_per_mille as { symbol: string; share: number }[]).filter((s) => /^[a-z]$/.test(s.symbol)).slice(0, 26);
const recipe = (data.language.corpus.recipe as { part: { corpus: string; weight: number }[] }).part.map((p) => ({ label: p.corpus.replace("prose-", ""), weight: p.weight }));
const atlas = data.search.atlas.points.map(([x, y, s]) => ({ x, y, s }));
const TYPING = typing(data.language.opening.split(",")[0], L.final, { wpm: 80 });
const FINAL_POSES = boardPosesAt(1);
const HOME = TYPING.rigs[1].home.index;

const Center = ({ children, bg = C.canvas }: { children: ReactNode; bg?: string }) =>
  <AbsoluteFill style={{ background: bg, alignItems: "center", justifyContent: "center" }}>{children}</AbsoluteFill>;

export const ITEMS: Item2D[] = [
  { title: "Board2D · start layout", file: "ui/Board2D.tsx", w: 1280, h: 560, el: () => <Center><Board2D u={56} chars={L.start} /></Center> },
  { title: "Board2D · final, fingers", file: "ui/Board2D.tsx", w: 1280, h: 560, el: () => <Center><Board2D u={56} chars={L.final} fill="fingers" /></Center> },
  { title: "Board2D · final, heat", file: "ui/Board2D.tsx", w: 1280, h: 560, el: () => <Center><Board2D u={56} chars={L.final} fill="heat" heat={heatN} /></Center> },
  {
    title: "LineChart", file: "ui/charts.tsx", w: 1000, h: 700, el: () => (
      <Center><Card title="Board search" note={`${B.snapshots.length} generations`}>
        <LineChart w={560} h={340} series={[{ points: B.convergence, step: true }]} highlight={B.convergence.at(-1) as [number, number]}
          threshold={{ y: B.start, label: "start" }} yFormat={(v) => v.toFixed(0)} xLabel="generation" />
      </Card></Center>
    ),
  },
  {
    title: "LineChart · SA workers", file: "ui/charts.tsx", w: 1000, h: 700, el: () => (
      <Center><LineChart w={600} h={400} yDomain={[9, 60]} series={L.workers[0].map((_, j) => ({ points: L.workers.map((w, i) => [i, w[j]] as [number, number]), width: 2 }))} /></Center>
    ),
  },
  {
    title: "Bars · finger load", file: "ui/charts.tsx", w: 1000, h: 700, el: () => (
      <Center><Card title="Finger load" note="% of presses">
        <Bars w={560} h={340} values={A.finger_load} labels={A.fingers} colors={A.fingers.map(fingerColor)} format={(v) => v.toFixed(0)} />
      </Card></Center>
    ),
  },
  {
    title: "Bars · letters", file: "ui/charts.tsx", w: 1000, h: 700, el: () => (
      <Center><Card title="Letters" note={data.corpus}><Bars w={560} h={340} values={letters.map((l) => l.share)} labels={letters.map((l) => l.symbol)} /></Card></Center>
    ),
  },
  { title: "Heatmap · letter bigrams", file: "ui/charts.tsx", w: 1000, h: 700, el: () => <Center><Card title="Letter bigrams" note={data.corpus}><Heatmap w={360} h={360} matrix={big.m} /></Card></Center> },
  {
    title: "Scatter · explore atlas", file: "ui/charts.tsx", w: 1000, h: 700, el: () => (
      <Center><Card title="Explore atlas" note={`${atlas.length} local optima`}><Scatter w={560} h={340} points={atlas} best={data.search.atlas.best} /></Card></Center>
    ),
  },
  { title: "EliteGrid · MAP-Elites", file: "ui/charts.tsx", w: 1000, h: 700, el: () => <Center><Card title="MAP-Elites" note="roll ratio × index load"><EliteGrid w={360} h={360} cells={data.search.qd.grid} /></Card></Center> },
  { title: "StackedBar · corpus recipe", file: "ui/charts.tsx", w: 1000, h: 400, el: () => <Center><StackedBar w={560} parts={recipe} /></Center> },
  { title: "RangeBars · board parameters", file: "ui/charts.tsx", w: 1100, h: 700, el: () => <Center><Card title="Board" note="free parameters" width={780}><RangeBars w={720} rows={ranges} /></Card></Center> },
  { title: "MetricCounter", file: "ui/numbers.tsx", w: 1000, h: 500, el: () => <Center><MetricCounter from={L.qwerty.score} to={L.best.score} count={1} label="layout score" accent size={TYPE.display * 0.8} /></Center> },
  {
    title: "StatRing", file: "ui/numbers.tsx", w: 1000, h: 560, el: () => (
      <Center><div style={{ display: "flex", gap: 60 }}>
        <StatRing value={L.best.sfb} max={4} label="SFB" unit="%" size={220} accent />
        <StatRing value={L.best.rolls} max={60} label="rolls" unit="%" size={220} />
      </div></Center>
    ),
  },
  {
    title: "DataTable", file: "ui/panels.tsx", w: 1100, h: 700, el: () => {
      const rows = data.board.keys.slice(0, 6).map((k, i) => [L.start[i] === "⇥" ? "Tab" : L.start[i].toUpperCase(), k.pose[0].toFixed(2), k.pose[1].toFixed(2), "0°", k.finger, k.effort.toFixed(1)]);
      return <Center><DataTable width={760} size={22} rows={rows}
        columns={[{ label: "key", align: "left" }, { label: "x" }, { label: "y" }, { label: "rot" }, { label: "finger", align: "left" }, { label: "effort" }]} /></Center>;
    },
  },
  {
    title: "MonoList", file: "ui/panels.tsx", w: 1100, h: 560, el: () => (
      <Center><Card title="Weights" note="profile hand" width={760} height={300}>
        <MonoList height={220} lines={Object.entries({ sfb: 2.867, pinky_ring: 1.176, hand_switch: 1.003, wrist: 1, thumb_arc: 1, outroll: 0.7151, bad_redirect: 0.5674, stretch: 0.5046 }).map(([k, v]) => [k, v.toFixed(3)])} />
      </Card></Center>
    ),
  },
  {
    title: "SegmentedTabs · StatusBadge", file: "ui/panels.tsx", w: 1100, h: 400, el: () => (
      <Center><div style={{ display: "flex", gap: 24, alignItems: "center" }}>
        <SegmentedTabs options={["Layout", "Board"]} active={0} />
        <StatusBadge kind="success" text="No collisions" />
        <StatusBadge kind="warning" text="Tight" />
        <StatusBadge kind="error" text="Collision" />
        <StatusBadge kind="best" text="Best" />
      </div></Center>
    ),
  },
  {
    title: "FileCard", file: "ui/panels.tsx", w: 1100, h: 400, el: () => (
      <Center><FileCard name="case.stl" size={`${Math.round(data.stl.case.bytes / 1024)} KB`} width={760}
        dims={data.stl.build.files[1].size_mm.map((v) => Math.round(v)).join(" × ") + " mm"} /></Center>
    ),
  },
  {
    title: "TerminalWindow", file: "ui/panels.tsx", w: 1100, h: 560, el: () => (
      <Center><TerminalWindow width={760} command="gecko perf --json" lines={[
        `full    ${fmt.int(data.perf.full_per_sec)} / s`, `delta   ${fmt.int(data.perf.delta_per_sec)} / s`,
        `build   ${fmt.int(data.perf.build_per_sec)} / s`, `rebuild ${fmt.int(data.perf.rebuild_per_sec)} / s`]} /></Center>
    ),
  },
  {
    title: "Eyebrow · KineticHeadline", file: "ui/type.tsx", w: 1280, h: 500, el: () => (
      <AbsoluteFill style={{ padding: 100, justifyContent: "center", gap: 24 }}>
        <Eyebrow text="Language." index="07" />
        <KineticHeadline text="Learned from what you type." width={1100} />
      </AbsoluteFill>
    ),
  },
  { title: "EndCard", file: "ui/panels.tsx", w: 1280, h: 720, el: () => <Center><EndCard /></Center> },
  { title: "Navbar", file: "ui/panels.tsx", w: 1600, h: 200, wide: true, el: () => <Center bg={C.soft}><div style={{ width: 1500 }}><Navbar active="layout" model="corne-42" score={L.best.score.toFixed(2)} status="Done" /></div></Center> },
  { title: "FlowChart · Home", file: "ui/panels.tsx", w: 1600, h: 800, wide: true, el: () => <Center><FlowChart width={1400} /></Center> },
  { title: "DocChrome", file: "ui/Document.tsx", w: 1600, h: 300, wide: true, el: () => <AbsoluteFill style={{ background: "#ebecef" }}><DocChrome title="Pride and Prejudice" width={1600} pageW={900} margin={90} /></AbsoluteFill> },
  { title: "LayoutPage · Optimize › Layout", file: "ui/LayoutPage.tsx", w: 1920, h: 1080, wide: true, el: () => <LayoutPage /> },
];

/** Drag to orbit, scroll to zoom, right-drag to pan. */
function Orbit({ cam, target }: { cam: [number, number, number]; target: [number, number, number] }) {
  const camera = useThree((s) => s.camera);
  useLayoutEffect(() => {
    camera.position.set(...cam);
    camera.lookAt(...target);
  }, [camera]); // eslint-disable-line react-hooks/exhaustive-deps
  return <OrbitControls makeDefault target={target} enableDamping={false} />;
}

export const MODELS: Model3D[] = [
  { title: "Corne", file: "three/Corne3D.tsx", cam: [-260, 320, 420], target: [0, 0, 0], el: () => <Corne3D chars={L.start} /> },
  { title: "Corne · exploded", file: "three/Corne3D.tsx", cam: [-260, 360, 420], target: [0, 20, 0], el: () => <Corne3D chars={L.start} explode={1} /> },
  { title: "Corne · optimized shape", file: "three/Corne3D.tsx", cam: [-380, 420, 520], target: [0, 20, 0], el: () => <Corne3D chars={L.final} poses={FINAL_POSES} fill="fingers" stack={0} tiles={1} lift={24} /> },
  { title: "STL case", file: "three/STLModel.tsx", cam: [-300, 380, 420], target: [0, 40, 0], el: () => <><STLModel /><Corne3D chars={L.final} poses={FINAL_POSES} stack={0} spread={0} lift={STL_KEY_LIFT} /></> },
  { title: "Hands on the board", file: "three/Hands3D.tsx", cam: [HOME[0] + 180, 560, HOME[2] + 820], target: [0, 40, 50], el: () => <><Corne3D chars={L.final} /><Hands3D typing={TYPING} t={-1} /></> },
  { title: "Hand skeleton", file: "three/Hands3D.tsx", cam: [HOME[0] + 180, 560, HOME[2] + 820], target: [0, 40, 50], el: () => <><Corne3D chars={L.final} /><Hands3D typing={TYPING} t={-1} skin={0} skeleton={1} /></> },
  { title: "Key + MX switch", file: "three/Switch.tsx", cam: [46, 52, 70], target: [0, 8, 0], floor: false, el: () => <Key size={[1, 1]} legend="f" color="#ffffff" ink={C.ink} homing /> },
  {
    title: "nice!nano, TRRS, reset", file: "three/NiceNano.tsx", cam: [40, 60, 70], target: [0, 0, 16], floor: false, el: () => (
      <><NiceNano /><group position={[-22, 0, 10]}><TRRS /></group><group position={[-22, 0, 30]}><ResetButton /></group></>
    ),
  },
];

/** A model in the promo's studio, with orbit controls in place of the scripted camera. */
export function ModelScene({ m }: { m: Model3D }) {
  return (
    <Scene3D floor={m.floor ?? true} fog={m.floor === false ? [400, 4000] : undefined}>
      <Orbit cam={m.cam} target={m.target} />
      {m.el()}
    </Scene3D>
  );
}

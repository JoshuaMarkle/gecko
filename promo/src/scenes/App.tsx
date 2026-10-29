// Scene 12: the whole product. The app (Optimize › Layout, rebuilt from the run) fills the frame and
// holds while the run plays, then tilts back into depth; one by one its parts lift off it at
// different heights: the explore atlas, MAP-Elites, the Home flowchart, and a terminal printing real
// `gecko perf` output.

import { AbsoluteFill } from "remotion";
import { data, fmt } from "../lib/data";
import { EASE, interp, progress, sec } from "../lib/motion";
import { C, H, W } from "../lib/tokens";
import { Card, EliteGrid, Scatter } from "../ui/charts";
import { LayoutPage } from "../ui/LayoutPage";
import { FlowChart, TerminalWindow } from "../ui/panels";
import { FloatingPanel, Space3D } from "../ui/space";
import { useLayout } from "./common";

const atlas = data.search.atlas.points.map(([x, y, s]) => ({ x, y, s }));

/** The terminal finishes typing at about 9.2 s; the scene holds on it before the end card. */
const T = { run: sec(3), tilt: sec(3.2), lift: sec(5.2), gap: sec(0.55), out: sec(10.8) };
/** Scene length (s) for the timeline. */
export const APP_S = (T.out + 14) / 30;

export function App() {
  const { frame } = useLayout();
  const tilt = progress(frame, T.tilt, 50, EASE.inOut);
  const drift = interp(frame, [[T.tilt, 0], [T.out + 10, 1]], EASE.linear);
  const out = progress(frame, T.out, 14, EASE.in);
  const rx = 20 * tilt, ry = -18 * tilt + 9 * drift, sc = 1 - 0.36 * tilt;
  const lift = (k: number) => progress(frame, T.lift + k * T.gap, 36, EASE.out);
  const panels: { x: number; y: number; z: number; el: React.ReactNode }[] = [
    { x: -650, y: -300, z: 320, el: <Card title="Explore" note={`${atlas.length} optima`} width={560}><Scatter w={500} h={300} points={atlas} best={data.search.atlas.best} /></Card> },
    { x: 700, y: -300, z: 250, el: <Card title="MAP-Elites" width={460}><EliteGrid w={400} h={300} cells={data.search.qd.grid} /></Card> },
    { x: 610, y: 330, z: 180, el: <div style={{ background: C.panel, border: `2px solid ${C.hair}`, borderRadius: 16, padding: 28 }}><FlowChart width={620} /></div> },
    { x: -560, y: 320, z: 380, el: <TerminalWindow width={700} size={28} command="gecko perf" reveal={progress(frame, T.lift + 3 * T.gap + 20, 50, EASE.linear)} lines={[
      `full     ${fmt.int(data.perf.full_per_sec).padStart(9)} / s`, `delta    ${fmt.int(data.perf.delta_per_sec).padStart(9)} / s`,
      `build    ${fmt.int(data.perf.build_per_sec).padStart(9)} / s`, `rebuild  ${fmt.int(data.perf.rebuild_per_sec).padStart(9)} / s`]} /> },
  ];
  return (
    <AbsoluteFill style={{ background: C.canvas, opacity: progress(frame, 0, 12, EASE.linear) * (1 - out) }}>
      <Space3D cam={{ perspective: 1600 }}>
        <div style={{ position: "absolute", transformStyle: "preserve-3d", transform: `translateZ(${-out * 600}px) rotateX(${rx}deg) rotateY(${ry}deg) scale(${sc})` }}>
          <FloatingPanel shadow={tilt}>
            <div style={{ width: W, height: H, boxShadow: `0 0 0 2px ${C.hair}` }}>
              <LayoutPage t={progress(frame, 0, T.run, EASE.inOut)} reveal={1} width={W} height={H} />
            </div>
          </FloatingPanel>
          {panels.map((p, i) => {
            const l = lift(i);
            return l > 0 && (
              <FloatingPanel key={i} x={p.x} y={p.y} z={p.z * l} opacity={Math.min(1, l * 2)} shadow={l}>{p.el}</FloatingPanel>
            );
          })}
        </div>
      </Space3D>
    </AbsoluteFill>
  );
}

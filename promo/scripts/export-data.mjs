// Export every number the promo shows into src/generated/promo-data.json, from real GECKO output:
// the CLI (`--json`), a `gecko serve` API (geometry and run JSON) and the product's WASM
// (`param_board` replays the board run's snapshots into key poses, `hand_costs` the hand model).
// Run from promo/: `npm run data`.
// Needs `make` (or `cargo build --release` + `npm run wasm:dev` in web/) first.

import { execFileSync, spawn } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "../..");
const OUT = resolve(HERE, "../src/generated/promo-data.json");
const PUB = resolve(HERE, "../public");
const GECKO = join(ROOT, "target/release/gecko");
const PORT = 7793;
const API = `http://127.0.0.1:${PORT}/api`;

const LAYOUT_RUN = "20261029-023718-sa-ed589d56";
const BOARD_RUN = "20261029-023950-geom-8e314d16";
const GEOMETRY = "corne-42";
const CORPUS = "prose";
const PROFILE = "hand";
const EXPLORE_RUN = "20261023-164330-explore-00e91488";
const QD_RUN = "20261022-031516-qd-af3fe56a";
const NSGA_RUN = "20261022-175309-nsga2-c91eb914";
const FINGERS = ["LP", "LR", "LM", "LI", "LT", "RT", "RI", "RM", "RR", "RP"];

const gecko = (...args) => JSON.parse(execFileSync(GECKO, ["--root", ROOT, ...args, "--json"], { encoding: "utf8", maxBuffer: 1 << 28 }));
const jsonl = (p) => readFileSync(p, "utf8").split("\n").filter(Boolean).map((l) => JSON.parse(l));
const r3 = (x) => Math.round(x * 1000) / 1000;

async function api(path) {
  const r = await fetch(API + path);
  if (!r.ok) throw new Error(`${path}: ${r.status}`);
  return r.json();
}

async function withServer(fn) {
  const srv = spawn(GECKO, ["--root", ROOT, "serve", "--port", String(PORT)], { stdio: "ignore" });
  try {
    for (let i = 0; ; i++) {
      try { await fetch(API + "/meta"); break; } catch { if (i > 100) throw new Error("gecko serve did not start"); await new Promise((r) => setTimeout(r, 100)); }
    }
    return await fn();
  } finally { srv.kill(); }
}

async function wasm() {
  const pkg = join(ROOT, "web/src/wasm/pkg");
  const mod = await import(join(pkg, "gecko_wasm.js"));
  mod.initSync({ module: readFileSync(join(pkg, "gecko_wasm_bg.wasm")) });
  return mod;
}

/** Per-key pose as `[x, y, z, rx, ry, rz]` (u, degrees), board-space (y toward the user, z up). */
const pose = (k) => [...k.pos, ...k.rot].map(r3);

/** The stats a ring gauge shows, from a `gecko score` breakdown. */
function stats(score) {
  const m = Object.fromEntries(score.breakdown.metrics.map((x) => [x.id, x.value]));
  return {
    score: r3(score.breakdown.score),
    sfb: r3(m.sfb), rolls: r3(m.inroll + m.outroll), alternate: r3(m.alternate),
    pinky_ring: r3(m.pinky_ring), reach: r3(m.reach), effort: r3(m.effort),
  };
}
const scoreChars = (chars) => stats(gecko("score", `raw:${chars}`, "-g", `${GEOMETRY}.param`, "-c", CORPUS, "-p", PROFILE));

/** `n` indices into `len` items, log-spaced so the early (fast-changing) part gets more of them. */
function logSample(len, n) {
  const out = new Set([0, len - 1]);
  for (let i = 0; i < n; i++) out.add(Math.min(len - 1, Math.round(Math.expm1((i / (n - 1)) * Math.log1p(len - 1)))));
  return [...out].sort((a, b) => a - b);
}

async function main() {
  const w = await wasm();
  const { geometry, boardRun, atlasGeometry } = await withServer(async () => {
    const all = await api("/geometries");
    const ecfg = JSON.parse(readFileSync(join(ROOT, "runs", EXPLORE_RUN, "config.json"), "utf8"));
    return { geometry: all.find((g) => g.id === GEOMETRY), boardRun: await api(`/runs/${BOARD_RUN}`), atlasGeometry: all.find((g) => g.id === ecfg.geometry.name) };
  });

  // --- board: the flat corne-42 ---
  const board = {
    name: geometry.name,
    description: geometry.description,
    keys: geometry.key.map((k) => ({ pose: pose(k), size: k.size, finger: k.finger, row: k.row, col: k.col, home: k.home, stretch: k.stretch, effort: r3(k.effort), fill: k.fill ?? null })),
    flat: geometry.flat.map((f) => [r3(f.pos[0]), r3(f.pos[1]), r3(f.rot)]),
  };

  // --- layout run: the SA trace ---
  const lcfg = JSON.parse(readFileSync(join(ROOT, "runs", LAYOUT_RUN, "config.json"), "utf8"));
  const lbest = JSON.parse(readFileSync(join(ROOT, "runs", LAYOUT_RUN, "best.json"), "utf8"));
  const lprog = jsonl(join(ROOT, "runs", LAYOUT_RUN, "progress.jsonl"));
  const steps = logSample(lprog.length, 160).map((i) => lprog[i]);
  const memo = new Map();
  const statsOf = (c) => { if (!memo.has(c)) memo.set(c, scoreChars(c)); return memo.get(c); };
  const finalChars = lprog.at(-1).best_chars;
  const layout = {
    run: LAYOUT_RUN, algo: lcfg.params.algo, seed: lcfg.params.seed, evals: lcfg.params.evals,
    start: lcfg.start, final: finalChars, pins: lcfg.pins,
    qwerty: statsOf(lcfg.start), best: statsOf(finalChars),
    snapshots: lprog.length,
    // `chars` is the worker's current layout (it moves every snapshot; best_chars has only ~40
    // distinct values), `best_chars` the best so far. Rings follow `chars`, the score follows `best`.
    trace: steps.map((p, i) => {
      const chars = i === steps.length - 1 ? p.best_chars : p.workers[0].chars;
      return { evals: p.evals, ms: p.elapsed_ms, best: r3(p.best), best_chars: p.best_chars, chars, current: r3(p.workers[0].current), stats: statsOf(chars) };
    }),
    workers: lprog.map((p) => p.workers.map((x) => r3(x.current))),
    temp: lprog.map((p) => p.workers.map((x) => r3(x.temp))),
    bestJson: { score: lbest.score ?? null },
  };

  // --- board run: replay each snapshot's parameters into key poses ---
  const names = boardRun.config.vars.map((v) => v.name);
  const gprog = jsonl(join(ROOT, "runs", BOARD_RUN, "progress.jsonl"));
  const replay = (values) => JSON.parse(w.param_board(JSON.stringify(boardRun.param), JSON.stringify(names), Float64Array.from(values)));
  const snaps = gprog.map((p) => {
    const r = replay(p.values);
    return { gen: p.generation, best: r3(p.best), collisions: p.collisions ?? r.collisions.length, poses: r.geometry.key.map(pose) };
  });
  const bvals = boardRun.result.best.values;
  const pick = (n) => ({ name: n, ...boardRun.config.vars.find((v) => v.name === n), value: r3(bvals[names.indexOf(n)]) });
  const buildJson = JSON.parse(readFileSync(join(ROOT, "runs", BOARD_RUN, "stl/build.json"), "utf8"));
  const boardOpt = {
    run: BOARD_RUN, boards: boardRun.config.params?.geom?.evals ?? null,
    start: r3(gprog[0].best), best: r3(boardRun.result.best.score ?? gprog.at(-1).best),
    snapshots: snaps,
    final: replay(bvals).geometry.key.map(pose),
    ranges: [pick("split.tent"), pick("fingers.row_curve"), pick("split.angle"), pick("thumbs.tilt")],
    convergence: gprog.map((p) => [p.generation, r3(p.best)]),
  };

  // --- STL files: copy into public/ and record sizes + bounds ---
  mkdirSync(join(PUB, "stl"), { recursive: true });
  const stl = {};
  for (const part of ["plate", "case", "bottom"]) {
    const src = join(ROOT, "runs", BOARD_RUN, "stl", `${part}.stl`);
    copyFileSync(src, join(PUB, "stl", `${part}.stl`));
    stl[part] = { file: `stl/${part}.stl`, bytes: statSync(src).size };
  }
  stl.build = buildJson;

  // --- hand model: the scorer's per-key hand costs and resting wrist posture (average hand, as
  // the Hand tab's Reach panel), and the product's skinned hand meshes ---
  const { flat: _flat, ...bare } = geometry;
  const hc = JSON.parse(w.hand_costs(JSON.stringify(bare), JSON.stringify({ size: "average" })));
  const hand = {
    size: "average", reach: hc.reach.map(r3), align: hc.align.map(r3), wrist: hc.wrist.map(r3), miss: hc.miss.map(r3),
    posture: hc.posture.map((p) => Object.fromEntries(Object.entries(p).map(([k, v]) => [k, r3(v)]))),
  };
  mkdirSync(join(PUB, "hands"), { recursive: true });
  for (const side of ["left", "right"]) copyFileSync(join(ROOT, "web/src/components/hand/assets", `${side}.glb`), join(PUB, "hands", `${side}.glb`));

  // --- search space: the Explore atlas map (WASM AtlasView, "fingers" embedding as in the
  // Explore tab), the MAP-Elites grid, the NSGA-II Pareto front ---
  const rd = (f) => new Uint8Array(readFileSync(join(ROOT, "runs", EXPLORE_RUN, f)));
  const view = new w.AtlasView(rd("optima.bin"), rd("edges.bin"), rd("samples.bin"));
  const fingerIdx = Uint8Array.from(atlasGeometry.key.map((k) => FINGERS.indexOf(k.finger)));
  const emb = view.embed("fingers", fingerIdx);
  const scores = view.base_scores();
  const atlasPoints = Array.from({ length: view.count() }, (_, i) => [r3(emb[2 * i]), r3(emb[2 * i + 1]), r3(scores[i])]);
  const qd = JSON.parse(readFileSync(join(ROOT, "runs", QD_RUN, "qd.json"), "utf8"));
  const [ax, ay] = qd.axes;
  const grid = Array.from({ length: ay.bins }, () => Array(ax.bins).fill(null));
  for (const e of qd.elites) grid[e.bins[1]][e.bins[0]] = r3(e.score);
  const nsga = JSON.parse(readFileSync(join(ROOT, "runs", NSGA_RUN, "best.json"), "utf8"));
  const objVal = (f, id) => f.breakdown.metrics.find((m) => m.id === id)?.value ?? null;
  const pareto = {
    objectives: nsga.objectives.map((o) => o.id),
    front: nsga.front.map((f) => [f.rank, ...nsga.objectives.map((o) => r3(objVal(f, o.id)))]),
  };
  const search = { atlas: { run: EXPLORE_RUN, points: atlasPoints, best: scores.indexOf(Math.min(...scores)) }, qd: { run: QD_RUN, axes: [ax, ay], grid }, pareto };

  // --- language, analysis, speed ---
  const corpus = gecko("corpus", "info", CORPUS);
  const books = gecko("corpus", "info", "prose-books");
  const analyze = gecko("analyze", `raw:${finalChars}`, "-g", `${GEOMETRY}.param`, "-c", CORPUS, "-p", PROFILE);
  const perf = gecko("perf");
  const metrics = gecko("list", "metrics");
  const profile = readFileSync(join(ROOT, "data/profiles/hand.toml"), "utf8");
  const novel = readFileSync(join(ROOT, "data/corpora/src/pg1342.txt"), "utf8");
  const opening = novel.slice(novel.indexOf("It is a truth universally")).split(/\n\s*\n/)[0].replace(/\s+/g, " ").trim();
  // Chapter I's first paragraphs for the document page (illustration captions and _italic_ marks dropped).
  const chapter = novel.slice(novel.indexOf("It is a truth universally")).split(/\n\s*\n/).map((x) => x.replace(/\s+/g, " ").trim())
    .filter((x) => x && !/^\[|^“He came down|\]$/.test(x)).slice(0, 14).map((x) => x.replace(/_/g, ""));
  const engram = readFileSync(join(ROOT, "data/comfort/engram-2key.csv"), "utf8").split("\n").slice(0, 40);

  const data = {
    generated: new Date().toISOString(), geometry: GEOMETRY, corpus: CORPUS, profile: PROFILE,
    board, layout, boardOpt, stl, perf, hand,
    language: { corpus, books, opening, chapter },
    analyze, metrics, profileToml: profile, engram, search,
  };
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, JSON.stringify(data));
  console.log(`wrote ${OUT} (${(statSync(OUT).size / 1024).toFixed(0)} KiB), ${memo.size} layouts scored`);
}

main().catch((e) => { console.error(e); process.exit(1); });

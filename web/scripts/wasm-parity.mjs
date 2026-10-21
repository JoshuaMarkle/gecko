// Exercise the browser's actual web-target WASM binary under Node, with fresh native
// CLI expectations. No server, browser, TOML reimplementation, or checked-in scores.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Engine, initSync } from "../src/wasm/pkg/gecko_wasm.js";

const root = fileURLToPath(new URL("../../", import.meta.url));
const build = execFileSync("cargo", [
  "build", "--release", "-p", "gecko-cli", "--bin", "gecko",
  "--example", "wasm_parity_fixture", "--message-format=json",
], { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
const artifacts = build.trim().split("\n").map(line => JSON.parse(line));
const executable = name => {
  const artifact = artifacts.find(item => item.reason === "compiler-artifact" && item.target.name === name && item.executable);
  assert.ok(artifact, `cargo did not produce ${name}`);
  return artifact.executable;
};
const fixtureJson = execFileSync(executable("wasm_parity_fixture"), [root, executable("gecko")], {
  cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024,
});
// Retain the precise CLI output for diagnosing failures locally and in CI artifacts.
const cache = new URL("../.cache/", import.meta.url);
mkdirSync(cache, { recursive: true });
writeFileSync(new URL("wasm-cli.json", cache), fixtureJson);
const fixture = JSON.parse(fixtureJson);
assert.ok(fixture.references.length > 0, "fixture must contain reference layouts");
assert.ok(fixture.cases.length > 0, "fixture must contain profiles");
initSync({ module: readFileSync(new URL("../src/wasm/pkg/gecko_wasm_bg.wasm", import.meta.url)) });
const corpus = readFileSync(fixture.corpus_path);
const relativeTolerance = 1e-6;

function close(actual, expected, label) {
  assert.ok(Number.isFinite(actual) && Number.isFinite(expected), `${label}: non-finite value`);
  // A truly zero expectation must stay zero; no absolute floor weakens small metrics.
  const tolerance = relativeTolerance * Math.abs(expected);
  assert.ok(Math.abs(actual - expected) <= tolerance,
    `${label}: WASM ${actual}, CLI ${expected}, relative tolerance ${relativeTolerance}`);
}

for (const { profile, expected } of fixture.cases) {
  assert.deepEqual(expected.layouts.map(layout => layout.layout).sort(), [...fixture.references].sort(),
    `${profile.name}: CLI fixture must include every reference exactly once`);
  for (const reference of expected.layouts) {
    test(`${profile.name} / ${reference.layout}: WASM matches CLI score and every metric`, () => {
      const engine = new Engine(JSON.stringify(fixture.geometry), corpus);
      try {
        const chars = reference.rows.join("").replaceAll(/\s/gu, "");
        const result = JSON.parse(engine.analyze(chars, JSON.stringify(profile), 0));
        assert.equal(result.schema, expected.schema);
        const actual = result.analysis.breakdown;
        close(actual.score, reference.breakdown.score, "total score");
        assert.deepEqual(actual.metrics.map(metric => metric.id), reference.breakdown.metrics.map(metric => metric.id));
        for (let i = 0; i < actual.metrics.length; i++) {
          const metric = actual.metrics[i];
          const native = reference.breakdown.metrics[i];
          close(metric.value, native.value, `${metric.id} value`);
          close(metric.weighted, native.weighted, `${metric.id} contribution`);
        }
      } finally {
        engine.free();
      }
    });
  }
}

// Render stills of a composition for review: `npm run stills -- <composition> <frame> [<frame> …]`.
// Writes out/<composition>-<frame>.png. Uses the system Chromium when present (no download).

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";

const [comp, ...frames] = process.argv.slice(2);
if (!comp || !frames.length) { console.error("usage: npm run stills -- <composition> <frame>…"); process.exit(2); }
const browser = ["/usr/bin/chromium", "/usr/sbin/chromium"].find(existsSync);
mkdirSync("out", { recursive: true });
for (const f of frames) {
  const args = ["remotion", "still", "src/index.ts", comp, `out/${comp}-${f}.png`, `--frame=${f}`, "--gl=angle", "--log=error"];
  if (browser) args.push(`--browser-executable=${browser}`);
  execFileSync("npx", args, { stdio: "inherit" });
}

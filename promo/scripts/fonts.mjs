// Copy the bundled fonts (Inter variable, JetBrains Mono; both OFL) from node_modules into public/fonts.
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const NM = resolve(HERE, "../node_modules");
const OUT = resolve(HERE, "../public/fonts");
mkdirSync(OUT, { recursive: true });
const files = {
  "inter-latin.woff2": "@fontsource-variable/inter/files/inter-latin-wght-normal.woff2",
  "inter-latin-ext.woff2": "@fontsource-variable/inter/files/inter-latin-ext-wght-normal.woff2",
  "mono-400.woff2": "@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2",
  "mono-500.woff2": "@fontsource/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff2",
};
for (const [to, from] of Object.entries(files)) copyFileSync(resolve(NM, from), resolve(OUT, to));
console.log(`copied ${Object.keys(files).length} fonts to ${OUT}`);

// Bundled fonts (public/fonts, `npm run fonts`): Inter variable and JetBrains Mono, both OFL.
// Loaded once through delayRender so no frame renders with fallback type.

import { continueRender, delayRender, staticFile } from "remotion";

const FACES: [string, string, string][] = [
  ["Inter", "fonts/inter-latin.woff2", "100 900"],
  ["Inter", "fonts/inter-latin-ext.woff2", "100 900"],
  ["JetBrains Mono", "fonts/mono-400.woff2", "400"],
  ["JetBrains Mono", "fonts/mono-500.woff2", "500"],
];

let loaded: Promise<void> | null = null;

export function loadFonts(): Promise<void> {
  if (loaded) return loaded;
  const handle = delayRender("fonts");
  loaded = Promise.all(FACES.map(async ([family, file, weight]) => {
    const f = new FontFace(family, `url(${staticFile(file)}) format("woff2")`, { weight });
    document.fonts.add(await f.load());
  })).then(() => continueRender(handle), (e) => { console.error(e); continueRender(handle); });
  return loaded;
}

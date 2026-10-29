// The component storyboard (components.html): every promo component in a Remotion Player, 3D ones
// with orbit controls. Remotion itself bundles with webpack (`npm run studio`); this is only the page.

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // three/hands.ts imports the product's web/src/hand.ts.
  server: { port: 7795, open: "/components.html", fs: { allow: [".."] } },
  build: { target: "es2022", outDir: "out/components", rollupOptions: { input: "components.html" }, chunkSizeWarningLimit: 8000 },
});

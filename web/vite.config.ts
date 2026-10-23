import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// In dev, the API comes from `gecko serve` (default port 7777).
export default defineConfig({
  plugins: [react()],
  assetsInclude: ["**/*.glb"],
  server: { proxy: { "/api": "http://127.0.0.1:7777" } },
  build: { target: "es2022", chunkSizeWarningLimit: 2000 },
});

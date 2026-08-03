import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  base: "./",
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    // Tauri may briefly restart its dev command after a backend rebuild. Do
    // not fail the whole GUI when the previous Vite process still owns 1420;
    // it can continue serving the already-running dev URL during the handoff.
    strictPort: false,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/target/**", "**/node_modules/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_PLATFORM,
    minify: process.env.TAURI_DEBUG ? false : "esbuild",
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});

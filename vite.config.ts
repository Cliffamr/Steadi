// ABOUTME: Builds the single trusted browser chrome page.
// ABOUTME: Remote browsing content lives in isolated native child webviews.

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  root: "src",
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    rollupOptions: {
      input: { main: resolve(__dirname, "src/main/index.html") },
      output: {
        manualChunks(id: string) {
          if (id.includes("/@tauri-apps/")) return "vendor-tauri";
          if (id.includes("/react-dom/") || id.includes("/react/") || id.includes("/scheduler/")) {
            return "vendor-react";
          }
        },
      },
    },
  },
});

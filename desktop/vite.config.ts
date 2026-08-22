import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";

// Tauri drives this dev server, so the port is fixed and failures must be loud rather than
// silently landing on another port the webview isn't pointed at.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": resolve(import.meta.dirname, "src") },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // The Rust side has its own rebuild loop; watching it would restart Vite for nothing.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // Matches the oldest webview Tauri targets on each desktop platform.
    target: "es2021",
    sourcemap: true,
  },
});

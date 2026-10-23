/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

import { katexWoff2Only } from "./vite-plugins";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [svelte(), katexWoff2Only()],
  // Keep Rust compiler errors visible in the terminal.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    // WKWebView on macOS 13+ is Safari 16+.
    target: "safari16",
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    chunkSizeWarningLimit: 2000,
  },
  test: {
    environment: "jsdom",
    include: ["tests/**/*.test.ts"],
  },
});

import { defineConfig } from "vite";

// Config Vite pour l'app Tauri v2 (front TS minimal, une seule fenetre : la popover).
// Port fixe requis par Tauri (devUrl dans tauri.conf.json). Port DELIBEREMENT distinct de 5173
// (occupe par le worktree de dev d'iakaTokenCounter, cf. specs/instructions/liste-des-agents.md
// § garde-fous) : documente aussi dans CLAUDE.md.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 5180,
    strictPort: true,
  },
  build: {
    target: "es2021",
  },
});

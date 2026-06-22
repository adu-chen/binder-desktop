import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 2 前端构建配置。X-SEQ-002：前端经 Service→command 触达后端，不跨层直连。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});

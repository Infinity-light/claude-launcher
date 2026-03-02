/**
 * ---
 * role: Vite 配置文件，配置 Vue 插件和开发服务器
 * depends:
 *   - @vitejs/plugin-vue
 * exports:
 *   - default config
 * status: PENDING
 * ---
 */

import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 5173,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));

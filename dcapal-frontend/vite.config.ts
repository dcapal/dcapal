import path from "path";
import { fileURLToPath } from "url";
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const isE2eMswMode = env.VITE_E2E_MSW === "1";
  const devProxy = isE2eMswMode
    ? undefined
    : {
      "/api/external/chart": {
        target:
          env.VITE_PROXY_YAHOO_CHART_TARGET ??
          "https://query1.finance.yahoo.com/v8/finance/chart/",
        changeOrigin: true,
        rewrite: (p: string) => p.replace(/^\/api\/external\/chart/, ""),
      },
      "/api/external/search": {
        target:
          env.VITE_PROXY_YAHOO_SEARCH_TARGET ??
          "https://query2.finance.yahoo.com/v1/finance/search",
        changeOrigin: true,
        rewrite: (p: string) => p.replace(/^\/api\/external\/search/, ""),
      },
      "/api": {
        target: env.VITE_PROXY_API_TARGET ?? "http://0.0.0.0:8080",
        changeOrigin: true,
        rewrite: (p: string) => p.replace(/^\/api/, ""),
      },
    };

  return {
    plugins: [react({ include: /\.[jt]sx?$/ }), wasm(), topLevelAwait()],
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "src"),
        "@app": path.resolve(__dirname, "src/app"),
        "@components": path.resolve(__dirname, "src/components"),
        "@demo": path.resolve(__dirname, "demo"),
        "@hooks": path.resolve(__dirname, "src/hooks"),
        "@images": path.resolve(__dirname, "images"),
        "@routes": path.resolve(__dirname, "src/routes"),
        "@utils": path.resolve(__dirname, "src/utils"),
        "@workers": path.resolve(__dirname, "src/workers"),
      },
    },
    server: {
      host: "0.0.0.0",
      port: 3000,
      strictPort: true,
      allowedHosts: true,
      headers: {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Headers": "*",
        "Access-Control-Allow-Methods": "*",
      },
      proxy: devProxy,
    },
    build: {
      sourcemap: true,
    },
    worker: {
      format: "iife",
      plugins: () => [wasm(), topLevelAwait()],
    },
    optimizeDeps: {
      exclude: ["dcapal-optimizer-wasm"],
    },
  };
});

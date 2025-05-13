import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  plugins: [
    react(),
    // wasm(),
    // topLevelAwait({
      // // The export name of top-level await promise for each chunk module
      // promiseExportName: "__devtla",
      // // The function to generate import names of top-level await promise in each chunk module
      // promiseImportName: (i) => `__devtla_${i}`,
    // }),
  ],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      "@app": fileURLToPath(new URL("./src/app", import.meta.url)),
      "@components": fileURLToPath(
        new URL("./src/components", import.meta.url)
      ),
      "@demo": fileURLToPath(new URL("./demo", import.meta.url)),
      "@hooks": fileURLToPath(new URL("./src/hooks", import.meta.url)),
      "@routes": fileURLToPath(new URL("./src/routes", import.meta.url)),
      "@utils": fileURLToPath(new URL("./src/utils", import.meta.url)),
      "@images": fileURLToPath(new URL("./images", import.meta.url)),
      "@workers": fileURLToPath(new URL("./workers", import.meta.url)),
    },
  },
  build: {
    rollupOptions: {
      output: { dir: "./dist", format: "iife", inlineDynamicImports: true },
      plugins: [
        // wasm(),
        // topLevelAwait({
          // // The export name of top-level await promise for each chunk module
          // promiseExportName: "__tla",
          // // The function to generate import names of top-level await promise in each chunk module
          // promiseImportName: (i) => `__tla_${i}`,
        // }),
      ],
    },
  },
  optimizeDeps: { exclude: ["@syntect/wasm", "dcapal-optimizer-wasm"] },
  worker: {
    rollupOptions: {
      output: { dir: "./dist", format: "iife", inlineDynamicImports: true },
    },
    plugins: () => [
      // wasm(),
      // topLevelAwait({
        // The export name of top-level await promise for each chunk module
        // promiseExportName: "__workertla",
        // // The function to generate import names of top-level await promise in each chunk module
        // promiseImportName: (i) => `____workertla_${i}`,
      // }),
    ],
  },
});

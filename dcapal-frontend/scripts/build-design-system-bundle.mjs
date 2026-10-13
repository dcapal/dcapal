// Builds a package-like distribution of the DcaPal design system for the
// design-sync (Storybook -> Claude Design) pipeline.
//
// The application has no library build, so this script produces one under
// `.design-sync/dist/` (gitignored):
//   index.js          ESM bundle of src/design-system/design-sync-entry.ts.
//                     Only first-party code is inlined; every npm import
//                     (react, @base-ui/react, lucide-react, recharts, ...)
//                     stays external and is bundled later by the converter.
//   index.css         preflight + design-system tokens/components, with the
//                     remote Google Fonts import hoisted first.
//   types/            .d.ts tree emitted from the same entry (props contracts).
//   package.json      names the dist so the converter treats it as the package.
//
// Run from `dcapal-frontend/`:  node scripts/build-design-system-bundle.mjs
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);
const distDir = path.join(frontendRoot, ".design-sync", "dist");
const entry = path.join(frontendRoot, "src/design-system/design-sync-entry.ts");
const appPackage = JSON.parse(
  readFileSync(path.join(frontendRoot, "package.json"), "utf8")
);

/** Loads esbuild from the app's node_modules, else from the staged converter deps. */
function loadEsbuild() {
  for (const base of [frontendRoot, path.join(frontendRoot, ".ds-sync")]) {
    try {
      return createRequire(path.join(base, "noop.js"))("esbuild");
    } catch {
      // try the next location
    }
  }
  throw new Error(
    "esbuild not found: run `npm i esbuild` in dcapal-frontend/.ds-sync first"
  );
}

const esbuild = loadEsbuild();

rmSync(distDir, { recursive: true, force: true });
mkdirSync(distDir, { recursive: true });

// 1. JS: first-party code only; npm packages stay external.
await esbuild.build({
  entryPoints: [entry],
  outfile: path.join(distDir, "index.js"),
  bundle: true,
  packages: "external",
  format: "esm",
  platform: "browser",
  target: "es2020",
  jsx: "automatic",
  logLevel: "warning",
});

// 2. CSS: Tailwind preflight (the app always ships it) + the design-system
// stylesheet. Preflight is compiled through the app's PostCSS pipeline so its
// `--theme()` functions resolve; esbuild then inlines the relative imports and
// hoists the remote font import to the top.
const postcss = createRequire(path.join(frontendRoot, "noop.js"))("postcss");
const tailwind = createRequire(path.join(frontendRoot, "noop.js"))(
  "@tailwindcss/postcss"
);
const preflightSource = [
  '@import "tailwindcss/theme.css" layer(theme);',
  '@import "tailwindcss/preflight.css" layer(base);',
].join("\n");
const preflight = await postcss([tailwind()]).process(preflightSource, {
  from: path.join(frontendRoot, "src/preflight.css"),
});
const preflightPath = path.join(distDir, "preflight.generated.css");
writeFileSync(preflightPath, preflight.css);
// The app's `body { @apply antialiased }` (src/style.css) is not shipped; mirror it
// on the theme scope (portaled overlays carry the class too) so text weight matches.
const ANTIALIAS_RULE =
  ".dcapal-theme{-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale}\n";
const dsStyles = path.join(frontendRoot, "src/design-system/styles/index.css");
await esbuild.build({
  stdin: {
    contents: `@import ${JSON.stringify(preflightPath)};\n@import ${JSON.stringify(dsStyles)};\n${ANTIALIAS_RULE}`,
    resolveDir: frontendRoot,
    loader: "css",
  },
  outfile: path.join(distDir, "index.css"),
  bundle: true,
  logLevel: "warning",
});
rmSync(preflightPath);

// 3. Type declarations for the props contracts, emitted from the same entry.
const tsconfigPath = path.join(distDir, "tsconfig.dts.json");
writeFileSync(
  tsconfigPath,
  JSON.stringify({
    extends: path.join(frontendRoot, "tsconfig.json"),
    compilerOptions: {
      noEmit: false,
      declaration: true,
      emitDeclarationOnly: true,
      rootDir: path.join(frontendRoot, "src"),
      outDir: path.join(distDir, "types"),
    },
    include: [],
    files: [entry],
  })
);
const tsc = spawnSync(
  process.execPath,
  [
    path.join(frontendRoot, "node_modules/typescript/bin/tsc"),
    "-p",
    tsconfigPath,
  ],
  { cwd: frontendRoot, encoding: "utf8" }
);
rmSync(tsconfigPath);
const typesEntry = "types/design-system/design-sync-entry.d.ts";
if (!existsSync(path.join(distDir, typesEntry))) {
  process.stderr.write(`${tsc.stdout}${tsc.stderr}`);
  throw new Error("tsc did not emit declarations for the design-sync entry");
}
if (tsc.status !== 0) {
  process.stderr.write(
    `warning: tsc reported type errors (declarations were still emitted)\n${tsc.stdout}`
  );
}

// 4. Package manifest the converter walks up to from the entry.
writeFileSync(
  path.join(distDir, "package.json"),
  `${JSON.stringify(
    {
      name: appPackage.name,
      version: appPackage.version,
      private: true,
      type: "module",
      module: "index.js",
      types: typesEntry,
    },
    null,
    2
  )}\n`
);

const files = readdirSync(distDir);
process.stdout.write(
  `design-system bundle written to ${path.relative(frontendRoot, distDir)} (${files.join(", ")})\n`
);

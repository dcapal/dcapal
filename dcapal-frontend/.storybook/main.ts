// This file has been automatically migrated to valid ESM format by Storybook.
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import path, { dirname } from "node:path";
import type { StorybookConfig } from "@storybook/react-webpack5";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const require = createRequire(import.meta.url);

const frontendRoot = path.resolve(__dirname, "..");

const config: StorybookConfig = {
  stories: ["../src/**/*.mdx", "../src/**/*.stories.@(js|jsx|mjs|ts|tsx)"],

  addons: [
    "@storybook/addon-a11y",
    "@storybook/addon-docs",
    "@storybook/addon-webpack5-compiler-babel",
  ],

  framework: {
    name: "@storybook/react-webpack5",
    options: {},
  },

  core: {
    disableTelemetry: true,
  },

  webpackFinal: async (webpackConfig) => {
    const cssRule = {
      test: /\.css$/,
      include: [path.resolve(frontendRoot, "src")],
      use: [
        "style-loader",
        {
          loader: "css-loader",
          options: { importLoaders: 1 },
        },
        {
          loader: "postcss-loader",
          options: {
            postcssOptions: {
              config: path.resolve(frontendRoot, "postcss.config.js"),
            },
          },
        },
      ],
    };

    const rules = webpackConfig.module?.rules ?? [];
    const rulesWithoutStorybookLoaders = rules.filter((rule) => {
      if (!rule || typeof rule !== "object" || !("test" in rule)) {
        return true;
      }

      const loaders = Array.isArray(rule.use)
        ? rule.use
        : [rule.use, rule.loader];
      const usesBabel = loaders.some((loader) => {
        if (typeof loader === "string") return loader.includes("babel-loader");
        if (!loader || typeof loader !== "object" || !("loader" in loader))
          return false;
        return (
          typeof loader.loader === "string" &&
          loader.loader.includes("babel-loader")
        );
      });

      return (
        !(rule.test instanceof RegExp && rule.test.test(".css")) && !usesBabel
      );
    });

    const babelRule = {
      test: /\.(?:c|m)?[jt]sx?$/,
      exclude: /node_modules/,
      use: {
        loader: require.resolve("babel-loader"),
        options: {
          babelrc: false,
          configFile: false,
          presets: [
            require.resolve("@babel/preset-env"),
            [require.resolve("@babel/preset-react"), { runtime: "automatic" }],
            require.resolve("@babel/preset-typescript"),
          ],
        },
      },
    };

    return {
      ...webpackConfig,
      module: {
        ...webpackConfig.module,
        rules: [...rulesWithoutStorybookLoaders, babelRule, cssRule],
      },
      resolve: {
        ...webpackConfig.resolve,
        alias: {
          ...webpackConfig.resolve?.alias,
          "@": path.resolve(frontendRoot, "src"),
          "@app": path.resolve(frontendRoot, "src/app"),
          "@components": path.resolve(frontendRoot, "src/components"),
          "@demo": path.resolve(frontendRoot, "demo"),
          "@design-system": path.resolve(frontendRoot, "src/design-system"),
          "@hooks": path.resolve(frontendRoot, "src/hooks"),
          "@images": path.resolve(frontendRoot, "images"),
          "@routes": path.resolve(frontendRoot, "src/routes"),
          "@utils": path.resolve(frontendRoot, "src/utils"),
          "@workers": path.resolve(frontendRoot, "src/workers"),
        },
      },
    };
  },
};

export default config;

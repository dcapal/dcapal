import { createRequire } from "node:module";

import { transformSync } from "@babel/core";
import { describe, expect, it } from "vitest";

const require = createRequire(import.meta.url);
const webpackCommon = require("../webpack.common.js");

const transformJsx = (mode: "development" | "production") => {
  const config = webpackCommon({}, { mode });
  const babelRule = config.module.rules.find((rule: any) => {
    return rule.test?.test?.("fixture.jsx");
  });

  return transformSync("const element = <div />;", {
    ...babelRule.options,
    // Webpack does not set Babel's envName from its mode. This reproduces the
    // default development transform used by the broken production build.
    envName: "development",
    filename: "fixture.jsx",
  })?.code;
};

/*
 * GIVEN a bundle built for production
 * WHEN Babel transforms its JSX
 * THEN it uses React's production JSX runtime, whose exports exist in the
 * production React package and do not call the development-only jsxDEV API.
 */
describe("Webpack JSX runtime configuration", () => {
  it("uses the production JSX runtime for production builds", () => {
    expect(transformJsx("production")).not.toContain("jsxDEV");
  });

  it("keeps the development JSX runtime for development builds", () => {
    expect(transformJsx("development")).toContain("jsxDEV");
  });
});

const { merge } = require("webpack-merge");
const common = require("./webpack.common.js");

const path = require("path");

const isE2eMode = process.env.REACT_APP_E2E_MSW === "1";
const backendPort = process.env.BACKEND_PORT || "8080";
const frontendPort = Number(process.env.FRONTEND_PORT || "3000");

module.exports = (env, argv) =>
  merge(common(env, argv), {
    mode: "development",
    devtool: "inline-source-map",
    devServer: {
      static: {
        directory: path.resolve(__dirname, "./dist"),
      },
      compress: true,
      hot: !isE2eMode,
      liveReload: !isE2eMode,
      allowedHosts: "all",
      port: frontendPort,
      historyApiFallback: true,
      hot: true,
      proxy: [
        {
          context: ["/api/external/chart"],
          target: "https://query1.finance.yahoo.com/v8/finance/chart/",
          pathRewrite: { "^/api/external/chart": "" },
          changeOrigin: true,
        },
        {
          context: ["/api/external/search"],
          target: "https://query2.finance.yahoo.com/v1/finance/search",
          pathRewrite: { "^/api/external/search": "" },
          changeOrigin: true,
        },
        {
          context: ["/api"],
          target: `http://127.0.0.1:${backendPort}`,
          pathRewrite: { "^/api": "" },
          changeOrigin: true,
        },
      ],
      headers: {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Headers": "*",
        "Access-Control-Allow-Methods": "*",
      },
    },
  });

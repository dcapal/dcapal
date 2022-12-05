const { merge } = require("webpack-merge");
const common = require("./webpack.common.js");

const path = require("path");

module.exports = (env, argv) =>
  merge(common(env, argv), {
    mode: "development",
    devtool: "inline-source-map",
    devServer: {
      contentBase: path.resolve(__dirname, "./dist"),
      hot: true,
      proxy: {
        "/api/yf/1": {
          target: "https://query1.finance.yahoo.com",
          pathRewrite: { "^/api/yf/1": "" },
          changeOrigin: true,
        },
        "/api/yf/2": {
          target: "https://query2.finance.yahoo.com",
          pathRewrite: { "^/api/yf/2": "" },
          changeOrigin: true,
        },
        "/api": {
          target: "http://localhost:8080",
          pathRewrite: { "^/api": "" },
          changeOrigin: true,
        },
      },
      headers: {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Headers": "*",
        "Access-Control-Allow-Methods": "*",
      },
    },
  });

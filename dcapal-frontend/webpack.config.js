const path = require("path");
const webpack = require("webpack");

const CopyWebpackPlugin = require("copy-webpack-plugin");
const ThreadsPlugin = require("threads-plugin");

module.exports = {
  entry: "./src/index.js",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bundle.js",
    publicPath: "/dist/",
  },
  mode: "development",
  module: {
    rules: [
      {
        test: /\.(js|jsx)$/,
        exclude: /node_modules/,
        loader: "babel-loader",
        options: { presets: ["@babel/env", "@babel/preset-react"] },
      },
      {
        test: /\.css$/i,
        include: path.resolve(__dirname, "src"),
        use: ["style-loader", "css-loader", "postcss-loader"],
      },
      {
        test: /\.svg$/,
        use: [
          {
            loader: "svg-url-loader",
            options: {
              limit: 10000,
            },
          },
        ],
      },
    ],
  },
  resolve: {
    extensions: ["*", ".js", ".jsx"],
  },
  plugins: [
    new CopyWebpackPlugin(["./src/index.html"]),
    new webpack.HotModuleReplacementPlugin(),
    new ThreadsPlugin(),
  ],
  devServer: {
    contentBase: path.resolve(__dirname, "./dist"),
    hot: true,
    proxy: {
      "/api/dcapal": {
        target: "http://localhost:8080",
        pathRewrite: { "^/api/dcapal": "" },
        changeOrigin: true,
      },
      "/api/cryptowatch": {
        target: "https://api.cryptowat.ch",
        pathRewrite: { "^/api/cryptowatch": "" },
        changeOrigin: true,
      },
      "/api/yfinance1": {
        target: "https://query1.finance.yahoo.com",
        pathRewrite: { "^/api/yfinance1": "" },
        changeOrigin: true,
      },
      "/api/yfinance2": {
        target: "https://query2.finance.yahoo.com",
        pathRewrite: { "^/api/yfinance2": "" },
        changeOrigin: true,
      },
    },
    headers: {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Headers": "*",
      "Access-Control-Allow-Methods": "*",
    },
  },
};

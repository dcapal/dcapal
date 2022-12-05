const path = require("path");
const webpack = require("webpack");

const CopyWebpackPlugin = require("copy-webpack-plugin");
const ThreadsPlugin = require("threads-plugin");

module.exports = (env, argv) => {
  const devMode = argv.mode !== "production";
  console.log("Webpack build mode:", argv.mode);
  console.log("  devMode:", devMode);

  return {
    entry: "./src/index.js",
    output: {
      path: path.resolve(__dirname, "dist"),
      filename: "bundle.js",
    },
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
      new CopyWebpackPlugin([
        { from: "./src/index.html", to: path.resolve(__dirname, "dist") },
      ]),
      new webpack.HotModuleReplacementPlugin(),
      new ThreadsPlugin(),
    ],
  };
};

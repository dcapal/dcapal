const path = require("path");
const webpack = require("webpack");

const HtmlWebpackPlugin = require("html-webpack-plugin");
const CopyWebpackPlugin = require("copy-webpack-plugin");

module.exports = (env, argv) => {
  const devMode = argv.mode !== "production";
  console.log("Webpack build mode:", argv.mode);
  console.log("  devMode:", devMode);

  return {
    output: {
      filename: "bundle.[contenthash].js",
      publicPath: "/",
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
          type: "asset/resource",
        },
      ],
    },
    resolve: {
      extensions: ["*", ".js", ".jsx"],
      alias: {
        "@images": path.resolve(__dirname, 'images/'),
      }
    },
    plugins: [
      new HtmlWebpackPlugin({
        title: "DcaPal - A smart assistant for your periodic investments",
        template: path.resolve(__dirname, "src", "index.html"),
      }),
      new CopyWebpackPlugin({
        patterns: [
          {
            from: path.resolve(__dirname, "static"),
            to: path.resolve(__dirname, "dist"),
          },
        ],
      }),
      new webpack.HotModuleReplacementPlugin(),
    ],
    experiments: {
      asyncWebAssembly: true,
      syncWebAssembly: true,
    },
  };
};

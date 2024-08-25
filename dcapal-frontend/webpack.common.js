const path = require("path");
const Dotenv = require("dotenv-webpack");

const HtmlWebpackPlugin = require("html-webpack-plugin");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const CssMinimizerPlugin = require("css-minimizer-webpack-plugin");

module.exports = (_env, argv) => {
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
          test: /.s?css$/,
          include: [
            path.resolve(__dirname, "src"),
            path.resolve(__dirname, "node_modules/vanilla-cookieconsent/dist"),
          ],
          use: [
            argv.mode === "production"
              ? MiniCssExtractPlugin.loader
              : "style-loader",
            {
              loader: "css-loader",
              options: { importLoaders: 1 },
            },
            "postcss-loader",
          ],
        },
        {
          test: /\.(svg|jpg|webp|png)$/,
          type: "asset/resource",
        },
      ],
    },
    resolve: {
      alias: {
        "@app": path.resolve(__dirname, "src/app/"),
        "@components": path.resolve(__dirname, "src/components"),
        "@demo": path.resolve(__dirname, "demo"),
        "@hooks": path.resolve(__dirname, "src/hooks"),
        "@images": path.resolve(__dirname, "images"),
        "@routes": path.resolve(__dirname, "src/routes"),
        "@utils": path.resolve(__dirname, "src/utils"),
        "@workers": path.resolve(__dirname, "src/workers"),
      },
    },
    plugins: [
      new Dotenv({
        path: `./.env.${
          argv.mode === "production" ? "production" : "development"
        }`,
      }),
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
      new MiniCssExtractPlugin({
        filename: "[name].[contenthash].css",
        chunkFilename: "[id].[contenthash].css",
      }),
    ],
    optimization: {
      minimizer: [
        new TerserPlugin(),
        new CssMinimizerPlugin({
          minimizerOptions: {
            preset: "advanced",
          },
        }),
      ],
    },
    experiments: {
      asyncWebAssembly: true,
      syncWebAssembly: true,
    },
  };
};

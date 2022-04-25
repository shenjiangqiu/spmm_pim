const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  experiments: {
    asyncWebAssembly: true,
  },
  entry: "./bootstrap.ts",
  devtool: 'inline-source-map',
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
  },
  mode: "development",
  plugins: [
    new CopyWebpackPlugin({ patterns: [{ from: 'index.html', to: "index.html" }] }),
  ],
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.js'],
  },
  devServer: {
    headers: {
      "Access-Control-Allow-Origin": "*",
    },
  },
};

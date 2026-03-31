const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');

const rootDir = __dirname;
const srcDir = path.resolve(rootDir, 'src');
const distDir = path.resolve(rootDir, 'dist');
const standaloneDir = path.resolve(rootDir, 'dist-standalone');
const isProd = process.env.NODE_ENV === 'production';
const buildApp = process.env.BUILD_APP === '1';

/**
 * Webpack config for @relyzer/client.
 *
 * Modes:
 * - default dev/build: library bundle for src/index.tsx -> dist/client.es.js
 * - BUILD_APP=1 build: standalone app for src/standalone.tsx -> dist-standalone/
 */
module.exports = {
  mode: isProd ? 'production' : 'development',
  target: 'web',
  devtool: isProd ? 'source-map' : 'eval-cheap-module-source-map',
  entry: buildApp
    ? path.resolve(srcDir, 'standalone.tsx')
    : path.resolve(srcDir, 'index.tsx'),
  output: {
    path: buildApp ? standaloneDir : distDir,
    filename: buildApp ? 'standalone.js' : 'client.es.js',
    library: buildApp
      ? undefined
      : {
          type: 'module',
        },
    module: !buildApp,
    chunkFormat: 'module',
    environment: {
      module: true,
    },
    clean: true,
    publicPath: '/',
  },
  experiments: {
    outputModule: !buildApp,
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.jsx', '.js'],
    alias: {
      '@': srcDir,
    },
  },
  externals: buildApp
    ? undefined
    : {
        react: 'react',
        'react-dom': 'react-dom',
        '@relyzer/shared': '@relyzer/shared',
        '@material-ui/styles': '@material-ui/styles',
        '@emotion/react': '@emotion/react',
        '@emotion/cache': '@emotion/cache',
        jss: 'jss',
      },
  module: {
    rules: [
      {
        test: /\.[jt]sx?$/,
        include: srcDir,
        use: [
          {
            loader: path.resolve(rootDir, 'webpack.relyzer-loader.js'),
          },
          {
            loader: require.resolve('babel-loader'),
            options: {
              babelrc: false,
              configFile: false,
              presets: [
                [require.resolve('@babel/preset-env'), { modules: false }],
                [require.resolve('@babel/preset-react'), { runtime: 'automatic', importSource: '@emotion/react' }],
                [require.resolve('@babel/preset-typescript'), { isTSX: true, allExtensions: true }],
              ],
            },
          },
        ],
      },
    ],
  },
  plugins: [
    ...(buildApp
      ? [
          new HtmlWebpackPlugin({
            template: path.resolve(rootDir, 'index.html'),
            inject: 'body',
          }),
        ]
      : []),
  ],
  devServer: {
    static: {
      directory: rootDir,
    },
    port: 8880,
    hot: true,
    open: false,
    client: {
      overlay: true,
    },
  },
  optimization: {
    minimize: buildApp && isProd,
  },
};

const { transformSync } = require('@babel/core');
const syntaxTs = require('@babel/plugin-syntax-typescript').default;
const relyzerBabel = require('@relyzer/babel').default;

module.exports = function relyzerWebpackLoader(source) {
  const callback = this.async();
  const filename = this.resourcePath;

  if (/node_modules|packages[\\/]client[\\/](dist|lib)/.test(filename)) {
    callback(null, source);
    return;
  }

  try {
    const result = transformSync(source, {
      filename,
      sourceMaps: this.sourceMap,
      babelrc: false,
      configFile: false,
      plugins: [
        [syntaxTs, { isTSX: /\.tsx$/i.test(filename) }],
        [relyzerBabel, { autoDetect: true }],
      ],
    });

    callback(null, result && result.code ? result.code : source, result && result.map ? result.map : undefined);
  } catch (error) {
    callback(error);
  }
};

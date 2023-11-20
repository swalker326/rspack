/**
 * @type {import('@rspack/core').RspackOptions}
 */
module.exports = {
	context: __dirname,
	entry: {
		main: "./index.js"
	},
	module: {
		rules: [
			{
				test: /index\.js$/,
				use: [
					{
						loader: "./my-loader.js"
					}
				]
			}
		]
	},
	optimization: {
		minimize: false
	},
	experiments: {
		rspackFuture: {
			newTreeshaking: true
		}
	}
};

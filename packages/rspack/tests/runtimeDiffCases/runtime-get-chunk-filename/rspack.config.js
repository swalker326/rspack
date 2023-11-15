module.exports = {
	output: {
		chunkFilename: "[name].[chunkhash:4].js",
		cssChunkFilename: "[name].css"
	},
	optimization: {
		splitChunks: {
			cacheGroups: {
				a: {
					test: /a.js/,
					minSize: 0
				},
				b: {
					test: /b.js/,
					minSize: 0
				}
			}
		}
	}
};

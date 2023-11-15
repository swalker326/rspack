module.exports = {
	output: {
		chunkFilename: "[name].[chunkhash:4].js",
		cssChunkFilename: "[name].css"
	},
	optimization: {
		// splitChunks: {
		//   cacheGroups: {
		//     a: {
		//       filename: 'a.[contenthash:4].js',
		//       test: /a.js/,
		//       minSize: 0,
		//     },
		//     b: {
		//       filename: 'b.[contenthash:4].js',
		//       test: /b.js/,
		//       minSize: 0,
		//     }
		//   },
		// }
	}
};

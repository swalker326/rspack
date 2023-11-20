const path = require("path");

// Rspack Loader
module.exports = function myLoader() {
	const callback = this.async();

	// specify the number
	process.env.NUM = 10;

	// execute at build time
	this.importModule(
		path.resolve(this.context, "fib.js"),
		{},
		(err, moduleExports) => {
			if (err) {
				throw err;
			}

			console.log(`Execute Result:\n${moduleExports}`);

			callback(null, `console.log(${moduleExports["default"]})`);
		}
	);
};

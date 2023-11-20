const path = require("path");

module.exports = function () {
	const callback = this.async();

	this.importModule(
		path.resolve(__dirname, "index.custom"),
		"",
		null,
		null,
		(res, err) => {
			console.log(res, err);

			callback({
				code: ""
			});
		}
	);
};

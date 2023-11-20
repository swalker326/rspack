import { RuntimeGlobals } from ".";
import type { Compiler } from "./Compiler";
import vm from "node:vm";

export default class ExecuteModulePlugin {
	constructor() {}

	apply(compiler: Compiler) {
		compiler.hooks.compilation.tap("executeModule", compilation => {
			compilation.hooks.executeModule.tap(
				"executeModule",
				(options, context) => {
					const moduleObject = options.moduleObject;
					const source = options.codeGenerationResult.get("javascript");

					console.log(source);
					console.log("========");

					const fn = vm.runInThisContext(
						`(function(module, __webpack_exports__, ${RuntimeGlobals.require}) {\n${source}\n})`,
						{
							filename: moduleObject.id
						}
					);

					fn.call(
						moduleObject.exports,
						moduleObject,
						moduleObject.exports,
						context.__webpack_require__
					);
				}
			);
		});
	}
}

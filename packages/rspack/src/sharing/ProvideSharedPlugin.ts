import { BuiltinPlugin, RawProvideOptions } from "@rspack/binding";
<<<<<<< HEAD
import { BuiltinPluginName, RspackBuiltinPlugin } from "../builtin-plugin/base";
=======
import {
	BuiltinPluginName,
	RspackBuiltinPlugin,
	create
} from "../builtin-plugin/base";
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
import { parseOptions } from "../container/options";
import { Compiler } from "../Compiler";
import { ModuleFederationRuntimePlugin } from "../container/ModuleFederationRuntimePlugin";

export type ProvideSharedPluginOptions = {
	provides: Provides;
	shareScope?: string;
};
export type Provides = (ProvidesItem | ProvidesObject)[] | ProvidesObject;
export type ProvidesItem = string;
export type ProvidesObject = {
	[k: string]: ProvidesConfig | ProvidesItem;
};
export type ProvidesConfig = {
	eager?: boolean;
	shareKey: string;
	shareScope?: string;
	version?: false | string;
};

export class ProvideSharedPlugin extends RspackBuiltinPlugin {
	name = BuiltinPluginName.ProvideSharedPlugin;
	_options: RawProvideOptions[];

	constructor(options: ProvideSharedPluginOptions) {
		super();
		this._options = parseOptions(
			options.provides,
			item => {
				if (Array.isArray(item))
					throw new Error("Unexpected array of provides");
				const result = {
					shareKey: item,
					version: undefined,
					shareScope: options.shareScope || "default",
					eager: false
				};
				return result;
			},
			item => ({
				shareKey: item.shareKey,
				version: item.version,
				shareScope: item.shareScope || options.shareScope || "default",
				eager: !!item.eager
			})
		).map(([key, v]) => ({ key, ...v }));
	}

	raw(compiler: Compiler): BuiltinPlugin {
		ModuleFederationRuntimePlugin.addPlugin(
			compiler,
<<<<<<< HEAD
			require.resolve("./initializeSharing.js")
=======
			require.resolve("../sharing/initializeSharing.js")
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
		);
		return {
			name: this.name as any,
			options: this._options
		};
	}
}

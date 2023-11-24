import { Compiler } from "../Compiler";
import { BuiltinPluginName, create } from "../builtin-plugin/base";
import { EntryPlugin } from "../builtin-plugin/EntryPlugin";

const ModuleFederationRuntimePlugin2 = create(
	BuiltinPluginName.ModuleFederationRuntimePlugin,
	() => undefined
);

<<<<<<< HEAD
const compilerToPlugins = new WeakMap<Compiler, Set<string>>();

export class ModuleFederationRuntimePlugin {
=======
export class ModuleFederationRuntimePlugin {
	plugins: string[] = [];

>>>>>>> c30ae9213 (feat: Module Federation, part 2, ContainerReferencePlugin (#4735))
	apply(compiler: Compiler) {
		// TODO: a hack to make sure this runtime is added after ContainerReferencePlugin
		// remove afterPlugin once we make rust side runtime_requirements_in_tree "tapable"
		compiler.hooks.afterPlugins.tap(
			{ name: ModuleFederationRuntimePlugin.name, stage: 10 },
			() => {
<<<<<<< HEAD
				const plugins = compilerToPlugins.get(compiler);
				if (plugins) {
					// TODO: move to rust side so don't depend on dataUrl?
					const entry = [...plugins]
						.map(p => `import ${JSON.stringify(p)};`)
						.join("\n");
					new EntryPlugin(compiler.context, `data:text/javascript,${entry}`, {
						name: undefined
					}).apply(compiler);
				}
=======
				// TODO: move to rust side so don't depend on dataUrl
				const entry = this.plugins.map(p => `import "${p}";`).join("\n");
				new EntryPlugin(compiler.context, `data:text/javascript,${entry}`, {
					name: undefined
				}).apply(compiler);
>>>>>>> c30ae9213 (feat: Module Federation, part 2, ContainerReferencePlugin (#4735))
				new ModuleFederationRuntimePlugin2().apply(compiler);
			}
		);
	}

<<<<<<< HEAD
	static addPlugin(compiler: Compiler, plugin: string) {
		let plugins = compilerToPlugins.get(compiler);
		if (!plugins) {
			compilerToPlugins.set(compiler, (plugins = new Set()));
		}
		plugins.add(plugin);
=======
	addPlugin(dep: string) {
		this.plugins.push(dep);
>>>>>>> c30ae9213 (feat: Module Federation, part 2, ContainerReferencePlugin (#4735))
	}
}

import { Compiler } from "../Compiler";
import { BuiltinPluginName, create } from "../builtin-plugin/base";
import { EntryPlugin } from "../builtin-plugin/EntryPlugin";

const ModuleFederationRuntimePlugin2 = create(
	BuiltinPluginName.ModuleFederationRuntimePlugin,
	() => undefined
);

<<<<<<< HEAD
const compilerToPlugins = new WeakMap<Compiler, Set<string>>();
=======
const compilerToPlugins = new WeakMap<Compiler, string[]>();
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))

export class ModuleFederationRuntimePlugin {
	apply(compiler: Compiler) {
		// TODO: a hack to make sure this runtime is added after ContainerReferencePlugin
		// remove afterPlugin once we make rust side runtime_requirements_in_tree "tapable"
		compiler.hooks.afterPlugins.tap(
			{ name: ModuleFederationRuntimePlugin.name, stage: 10 },
			() => {
				const plugins = compilerToPlugins.get(compiler);
				if (plugins) {
					// TODO: move to rust side so don't depend on dataUrl?
<<<<<<< HEAD
					const entry = [...plugins]
						.map(p => `import ${JSON.stringify(p)};`)
						.join("\n");
=======
					const entry = plugins.map(p => `import "${p}";`).join("\n");
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
					new EntryPlugin(compiler.context, `data:text/javascript,${entry}`, {
						name: undefined
					}).apply(compiler);
				}
				new ModuleFederationRuntimePlugin2().apply(compiler);
			}
		);
	}

	static addPlugin(compiler: Compiler, plugin: string) {
		let plugins = compilerToPlugins.get(compiler);
		if (!plugins) {
<<<<<<< HEAD
			compilerToPlugins.set(compiler, (plugins = new Set()));
		}
		plugins.add(plugin);
=======
			compilerToPlugins.set(compiler, (plugins = []));
		}
		plugins.push(plugin);
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
	}
}

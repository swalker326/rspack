// use std::sync::Arc;

// use async_trait::async_trait;
// use rspack_error::Result;
// use rspack_util::fx_dashmap::{FxDashMap, FxDashSet};
// use tokio::sync::{mpsc::UnboundedSender, Mutex, RwLock};

// use crate::{
//   BoxModule, BuildTimeExecutionOption, BuildTimeExecutionTask, CompilationArgs, DependencyId,
//   DependencyType, FactorizeArgs, FactorizeTask, ModuleIdentifier, NormalModuleCreateData,
//   NormalModuleFactoryContext, Plugin, PluginContext, PluginFactorizeHookOutput,
//   PluginNormalModuleFactoryModuleHookOutput, SucceedModuleArgs,
// };

// /// In this plugin we remember all built modules, and all its children,
// /// like we did in module_graph.
// /// If one triggers execute_module, the trigger point is a dependency,
// /// let us called it root dependency.
// /// If we want to execute one module, we need to know all its sub modules,
// /// its children and children's children, after all those modules get built
// /// we can sure we are safe to execute them.
// #[derive(Debug, Default)]
// pub struct PluginBuildTimeExecutionModule {
//   /// This is a map representing root_dependency -> its sub modules.
//   /// Every time we entry one module, we remove the module from it,
//   /// and insert all module children into it, so if this set becomes
//   /// empty, that means all children are built
//   buildtime_modules: FxDashMap<DependencyId, FxDashSet<DependencyId>>,

//   /// For performance reason, we store the module dependency id and
//   /// its root dependencies, so we can know if one module belongs to
//   /// any root
//   id_to_root: FxDashMap<DependencyId, FxDashSet<DependencyId>>,
//   dep_to_module: FxDashMap<DependencyId, ModuleIdentifier>,
//   module_to_dep: FxDashMap<ModuleIdentifier, DependencyId>,

//   built_modules: FxDashSet<DependencyId>,

//   // store the map for dependency -> import_module sender
//   dep_to_task: FxDashMap<DependencyId, (BuildTimeExecutionOption, UnboundedSender<Result<String>>)>,

//   // buildtime_modules: FxDashMap<DependencyId, FxDashSet<ModuleIdentifier>>,
//   factorize_queue: Arc<Mutex<Option<UnboundedSender<FactorizeTask>>>>,
//   buildtime_execution_queue: Arc<Mutex<Option<UnboundedSender<BuildTimeExecutionTask>>>>,

//   lock: Arc<RwLock<()>>,
// }

// impl PluginBuildTimeExecutionModule {
//   pub fn new() -> Self {
//     Self::default()
//   }
// }

// impl PluginBuildTimeExecutionModule {
//   fn get_dep_id(&self, m: &ModuleIdentifier) -> DependencyId {
//     *self.module_to_dep.get(m).expect("should have dependency")
//   }

//   fn get_module_id(&self, d: &DependencyId) -> ModuleIdentifier {
//     *self.dep_to_module.get(d).expect("should have dependency")
//   }

//   fn clear_state(&self) {
//     self.buildtime_modules.clear();
//     self.id_to_root.clear();
//     self.dep_to_module.clear();
//     self.module_to_dep.clear();
//     self.built_modules.clear();
//     self.dep_to_task.clear();
//   }
// }

// #[async_trait]
// impl Plugin for PluginBuildTimeExecutionModule {
//   async fn compilation(&self, args: CompilationArgs<'_>) -> Result<()> {
//     if !self.buildtime_modules.is_empty() {
//       self.clear_state();
//     }

//     let compilation = args.compilation;
//     let mut factorize_queue = self.factorize_queue.lock().await;
//     *factorize_queue = Some(compilation.factorize_queue.get_tx());
//     let mut buildtime_execution_queue = self.buildtime_execution_queue.lock().await;
//     *buildtime_execution_queue = Some(compilation.buildtime_execution_queue.get_tx());

//     Ok(())
//   }

//   async fn factorize(
//     &self,
//     _ctx: PluginContext,
//     factorize_args: FactorizeArgs<'_>,
//     _job_ctx: &mut NormalModuleFactoryContext,
//   ) -> PluginFactorizeHookOutput {
//     if matches!(
//       factorize_args.dependency.dependency_type(),
//       DependencyType::LoaderImport
//     ) {
//       let dep_id = *factorize_args.dependency.id();
//       self
//         .buildtime_modules
//         .entry(dep_id)
//         .or_default()
//         .insert(dep_id);

//       self.id_to_root.entry(dep_id).or_default().insert(dep_id);
//     }

//     Ok(None)
//   }

//   async fn normal_module_factory_module(
//     &self,
//     _ctx: PluginContext,
//     module: BoxModule,
//     args: &NormalModuleCreateData,
//   ) -> PluginNormalModuleFactoryModuleHookOutput {
//     self
//       .module_to_dep
//       .insert(module.identifier(), args.dependency_id);

//     self
//       .dep_to_module
//       .insert(args.dependency_id, module.identifier());

//     Ok(module)
//   }

//   async fn succeed_module(&self, args: &SucceedModuleArgs<'_>) -> Result<()> {
//     let read = self.lock.read().await;
//     let dep_id = self.get_dep_id(&args.module.identifier());

//     self.built_modules.insert(dep_id);

//     if let Some(root_dependencies) = &self.id_to_root.get(&dep_id) {
//       drop(read);
//       let write = self.lock.write().await;
//       // this module is some root dependencies' children
//       for root in root_dependencies.iter() {
//         // find all root
//         let root_id = *root;
//         drop(root);

//         // remove self from the queue and add all children into the queue
//         let remaining_modules = self
//           .buildtime_modules
//           .get(&root_id)
//           .expect("should have remaining modules");

//         remaining_modules.remove(&dep_id);

//         let build_result = args.build_result;

//         for dep in build_result.dependencies.iter().filter(|dep| {
//           matches!(
//             dep.dependency_type(),
//             DependencyType::EsmImport(_)
//               | DependencyType::CjsRequire
//               | DependencyType::ImportContext
//               | DependencyType::CommonJSRequireContext
//               | DependencyType::WasmImport
//           )
//         }) {
//           let id = *dep.id();
//           self.id_to_root.entry(id).or_default().insert(root_id);

//           if self.built_modules.contains(&id) {
//             // this module already been built
//             continue;
//           }

//           remaining_modules.insert(id);
//         }

//         // all the modules have been built, we can start execute
//         if remaining_modules.is_empty() {
//           let lock = self.buildtime_execution_queue.lock().await;
//           let queue = lock
//             .as_ref()
//             .expect("should have buildtime execution queues");

//           let data = self.dep_to_task.get(&root_id).expect("should have data");

//           queue
//             .send(BuildTimeExecutionTask::Ready(
//               crate::BuildTimeExecutionReady {
//                 options: data.0.clone(),
//                 sender: data.1.clone(),
//                 module: self.get_module_id(&root_id),
//               },
//             ))
//             .expect("failed to send buildtime execution task");
//         }
//       }

//       drop(write);
//     }

//     Ok(())
//   }

//   fn prepare_execute_module(
//     &self,
//     dep: DependencyId,
//     options: &BuildTimeExecutionOption,
//     sender: UnboundedSender<Result<String>>,
//   ) -> Result<()> {
//     self.dep_to_task.insert(dep, (options.clone(), sender));

//     Ok(())
//   }
// }

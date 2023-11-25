use std::sync::Arc;

use rspack_error::{Diagnostic, Result};
use rustc_hash::FxHashMap;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::{
  cache::Cache, BoxDependency, BuildContext, BuildResult, Compilation, CompilerContext,
  CompilerOptions, Context, ContextModuleFactory, DependencyType, Module, ModuleFactory,
  ModuleFactoryCreateData, ModuleFactoryResult, ModuleGraph, ModuleGraphModule, ModuleIdentifier,
  ModuleProfile, ModuleType, NormalModuleFactory, NormalModuleFactoryContext, Resolve,
  ResolverFactory, SharedPluginDriver, WorkerQueue,
};
use crate::{BoxModule, DependencyId, ExportInfo, ExportsInfo, SucceedModuleArgs, UsageState};

#[derive(Debug)]
pub enum TaskResult {
  Factorize(Box<FactorizeTaskResult>),
  Add(Box<AddTaskResult>),
  Build(Box<BuildTaskResult>),
  ProcessDependencies(Box<ProcessDependenciesResult>),
}

#[async_trait::async_trait]
pub trait WorkerTask {
  async fn run(self) -> Result<TaskResult>;
}

pub struct FactorizeTask {
  pub original_module_identifier: Option<ModuleIdentifier>,
  pub original_module_context: Option<Box<Context>>,
  pub issuer: Option<Box<str>>,
  pub dependency: BoxDependency,
  pub dependencies: Vec<DependencyId>,
  pub is_entry: bool,
  pub module_type: Option<ModuleType>,
  pub side_effects: Option<bool>,
  pub resolve_options: Option<Box<Resolve>>,
  pub resolver_factory: Arc<ResolverFactory>,
  pub loader_resolver_factory: Arc<ResolverFactory>,
  pub options: Arc<CompilerOptions>,
  pub lazy_visit_modules: std::collections::HashSet<String>,
  pub plugin_driver: SharedPluginDriver,
  pub cache: Arc<Cache>,
  pub current_profile: Option<Box<ModuleProfile>>,
  pub callback: Option<ModuleCreationCallback>,
}

impl std::fmt::Debug for FactorizeTask {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("FactorizeTask")
      .field(
        "original_module_identifier",
        &self.original_module_identifier,
      )
      .field("original_module_context", &self.original_module_context)
      .field("issuer", &self.issuer)
      .field("dependency", &self.dependency)
      .field("dependencies", &self.dependencies)
      .field("is_entry", &self.is_entry)
      .field("module_type", &self.module_type)
      .field("side_effects", &self.side_effects)
      .field("resolve_options", &self.resolve_options)
      .field("resolver_factory", &self.resolver_factory)
      .field("loader_resolver_factory", &self.loader_resolver_factory)
      .field("options", &self.options)
      .field("lazy_visit_modules", &self.lazy_visit_modules)
      .field("plugin_driver", &self.plugin_driver)
      .field("cache", &self.cache)
      .field("current_profile", &self.current_profile)
      .finish()
  }
}

/// a struct temporarily used creating ExportsInfo
#[derive(Debug)]
pub struct ExportsInfoRelated {
  pub exports_info: ExportsInfo,
  pub other_exports_info: ExportInfo,
  pub side_effects_info: ExportInfo,
}

pub struct FactorizeTaskResult {
  pub dependency: DependencyId,
  pub original_module_identifier: Option<ModuleIdentifier>,
  pub factory_result: ModuleFactoryResult,
  pub module_graph_module: Box<ModuleGraphModule>,
  pub dependencies: Vec<DependencyId>,
  pub diagnostics: Vec<Diagnostic>,
  pub is_entry: bool,
  pub current_profile: Option<Box<ModuleProfile>>,
  pub exports_info_related: ExportsInfoRelated,
  pub from_cache: bool,
  pub callback: Option<ModuleCreationCallback>,
}

impl std::fmt::Debug for FactorizeTaskResult {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("FactorizeTaskResult")
      .field(
        "original_module_identifier",
        &self.original_module_identifier,
      )
      .field("factory_result", &self.factory_result)
      .field("module_graph_module", &self.module_graph_module)
      .field("dependencies", &self.dependencies)
      .field("diagnostics", &self.diagnostics)
      .field("is_entry", &self.is_entry)
      .field("current_profile", &self.current_profile)
      .field("exports_info_related", &self.exports_info_related)
      .field("from_cache", &self.from_cache)
      .finish()
  }
}

#[async_trait::async_trait]
impl WorkerTask for FactorizeTask {
  async fn run(self) -> Result<TaskResult> {
    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_factory_start();
    }
    let dependency = self.dependency;
    let dep_id = *dependency.id();

    let context = if let Some(context) = dependency.get_context() {
      context
    } else if let Some(context) = &self.original_module_context {
      context
    } else {
      &self.options.context
    }
    .clone();

    let (result, diagnostics) = match *dependency.dependency_type() {
      DependencyType::ImportContext
      | DependencyType::CommonJSRequireContext
      | DependencyType::RequireContext
      | DependencyType::ImportMetaContext => {
        assert!(dependency.as_module_dependency().is_none());
        let factory = ContextModuleFactory::new(self.plugin_driver, self.cache);
        factory
          .create(ModuleFactoryCreateData {
            resolve_options: self.resolve_options,
            context,
            dependency,
          })
          .await?
          .split_into_parts()
      }
      DependencyType::ContainerEntry => {
        let factory = crate::mf::container_entry_module_factory::ContainerEntryModuleFactory;
        factory
          .create(ModuleFactoryCreateData {
            resolve_options: self.resolve_options,
            context,
            dependency,
          })
          .await?
          .split_into_parts()
      }
      DependencyType::ProvideSharedModule => {
        let factory = crate::mf::provide_shared_module_factory::ProvideSharedModuleFactory;
        factory
          .create(ModuleFactoryCreateData {
            resolve_options: self.resolve_options,
            context,
            dependency,
          })
          .await?
          .split_into_parts()
      }
      _ => {
        assert!(dependency.as_context_dependency().is_none());
        let factory = NormalModuleFactory::new(
          NormalModuleFactoryContext {
            original_module_identifier: self.original_module_identifier,
            module_type: self.module_type,
            side_effects: self.side_effects,
            options: self.options.clone(),
            lazy_visit_modules: self.lazy_visit_modules,
            issuer: self.issuer,
          },
          self.loader_resolver_factory,
          self.plugin_driver,
          self.cache,
        );
        factory
          .create(ModuleFactoryCreateData {
            resolve_options: self.resolve_options,
            context,
            dependency,
          })
          .await?
          .split_into_parts()
      }
    };

    let other_exports_info = ExportInfo::new(None, UsageState::Unknown, None);
    let side_effects_only_info = ExportInfo::new(
      Some("*side effects only*".into()),
      UsageState::Unknown,
      None,
    );
    let exports_info = ExportsInfo::new(other_exports_info.id, side_effects_only_info.id);
    let mgm = ModuleGraphModule::new(
      result.module.identifier(),
      *result.module.module_type(),
      exports_info.id,
    );

    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_factory_end();
    }

    Ok(TaskResult::Factorize(Box::new(FactorizeTaskResult {
      dependency: dep_id,
      is_entry: self.is_entry,
      original_module_identifier: self.original_module_identifier,
      from_cache: result.from_cache,
      factory_result: result,
      module_graph_module: Box::new(mgm),
      dependencies: self.dependencies,
      diagnostics,
      current_profile: self.current_profile,
      exports_info_related: ExportsInfoRelated {
        exports_info,
        other_exports_info,
        side_effects_info: side_effects_only_info,
      },
      callback: self.callback,
    })))
  }
}

pub type FactorizeQueue = WorkerQueue<FactorizeTask>;

pub struct AddTask {
  pub original_module_identifier: Option<ModuleIdentifier>,
  pub module: Box<dyn Module>,
  pub module_graph_module: Box<ModuleGraphModule>,
  pub dependencies: Vec<DependencyId>,
  pub is_entry: bool,
  pub current_profile: Option<Box<ModuleProfile>>,
  pub callback: Option<ModuleCreationCallback>,
}

impl std::fmt::Debug for AddTask {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("AddTask")
      .field(
        "original_module_identifier",
        &self.original_module_identifier,
      )
      .field("module", &self.module)
      .field("module_graph_module", &self.module_graph_module)
      .field("dependencies", &self.dependencies)
      .field("is_entry", &self.is_entry)
      .field("current_profile", &self.current_profile)
      .finish()
  }
}

#[derive(Debug)]
pub enum AddTaskResult {
  ModuleReused {
    module: Box<dyn Module>,
  },
  ModuleAdded {
    module: Box<dyn Module>,
    current_profile: Option<Box<ModuleProfile>>,
  },
}

impl AddTask {
  pub fn run(self, compilation: &mut Compilation) -> Result<TaskResult> {
    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_integration_start();
    }
    let module_identifier = self.module.identifier();

    if compilation
      .module_graph
      .module_graph_module_by_identifier(&module_identifier)
      .is_some()
    {
      set_resolved_module(
        &mut compilation.module_graph,
        self.original_module_identifier,
        self.dependencies,
        module_identifier,
      )?;

      if let Some(callback) = self.callback {
        callback(&self.module);
      }

      return Ok(TaskResult::Add(Box::new(AddTaskResult::ModuleReused {
        module: self.module,
      })));
    }

    compilation
      .module_graph
      .add_module_graph_module(*self.module_graph_module);

    set_resolved_module(
      &mut compilation.module_graph,
      self.original_module_identifier,
      self.dependencies,
      module_identifier,
    )?;

    if self.is_entry {
      compilation
        .entry_module_identifiers
        .insert(module_identifier);
    }

    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_integration_end();
    }

    if let Some(callback) = self.callback {
      callback(&self.module);
    }

    Ok(TaskResult::Add(Box::new(AddTaskResult::ModuleAdded {
      module: self.module,
      current_profile: self.current_profile,
    })))
  }
}

fn set_resolved_module(
  module_graph: &mut ModuleGraph,
  original_module_identifier: Option<ModuleIdentifier>,
  dependencies: Vec<DependencyId>,
  module_identifier: ModuleIdentifier,
) -> Result<()> {
  for dependency in dependencies {
    module_graph.set_resolved_module(original_module_identifier, dependency, module_identifier)?;
  }
  Ok(())
}

pub type AddQueue = WorkerQueue<AddTask>;

#[derive(Debug)]
pub struct BuildTask {
  pub module: Box<dyn Module>,
  pub resolver_factory: Arc<ResolverFactory>,
  pub compiler_options: Arc<CompilerOptions>,
  pub plugin_driver: SharedPluginDriver,
  pub cache: Arc<Cache>,
  pub current_profile: Option<Box<ModuleProfile>>,
}

#[derive(Debug)]
pub struct BuildTaskResult {
  pub module: Box<dyn Module>,
  pub build_result: Box<BuildResult>,
  pub diagnostics: Vec<Diagnostic>,
  pub current_profile: Option<Box<ModuleProfile>>,
  pub from_cache: bool,
}

#[async_trait::async_trait]
impl WorkerTask for BuildTask {
  async fn run(self) -> Result<TaskResult> {
    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_building_start();
    }

    let mut module = self.module;
    let compiler_options = self.compiler_options;
    let resolver_factory = self.resolver_factory;
    let cache = self.cache;
    let plugin_driver = self.plugin_driver;

    let (build_result, is_cache_valid) = match cache
      .build_module_occasion
      .use_cache(&mut module, |module| async {
        plugin_driver
          .build_module(module.as_mut())
          .await
          .unwrap_or_else(|e| panic!("Run build_module hook failed: {}", e));

        let result = module
          .build(BuildContext {
            compiler_context: CompilerContext {
              options: compiler_options.clone(),
              resolver_factory: resolver_factory.clone(),
              module: Some(module.identifier()),
              module_context: module.as_normal_module().and_then(|m| m.get_context()),
            },
            plugin_driver: plugin_driver.clone(),
            compiler_options: &compiler_options,
          })
          .await;

        result.map(|t| (t, module))
      })
      .await
    {
      Ok(result) => result,
      Err(err) => panic!("build module get error: {}", err),
    };

    if is_cache_valid {
      plugin_driver.still_valid_module(module.as_ref()).await?;
    }

    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_building_end();
    }

    if let Ok(build_result) = &build_result {
      plugin_driver
        .succeed_module(&SucceedModuleArgs {
          module: &module,
          build_result: &build_result.inner,
        })
        .await
        .unwrap_or_else(|e| panic!("Run succeed_module hook failed: {}", e));
    }

    build_result.map(|build_result| {
      let (build_result, diagnostics) = build_result.split_into_parts();

      TaskResult::Build(Box::new(BuildTaskResult {
        module,
        build_result: Box::new(build_result),
        diagnostics,
        current_profile: self.current_profile,
        from_cache: is_cache_valid,
      }))
    })
  }
}

pub type BuildQueue = WorkerQueue<BuildTask>;

#[derive(Debug)]
pub struct ProcessDependenciesTask {
  pub original_module_identifier: ModuleIdentifier,
  pub dependencies: Vec<DependencyId>,
  pub resolve_options: Option<Box<Resolve>>,
}

#[derive(Debug)]
pub struct ProcessDependenciesResult {
  pub module_identifier: ModuleIdentifier,
}

pub type ProcessDependenciesQueue = WorkerQueue<ProcessDependenciesTask>;

pub struct CleanTask {
  pub module_identifier: ModuleIdentifier,
}

#[derive(Debug)]
pub enum CleanTaskResult {
  ModuleIsUsed {
    module_identifier: ModuleIdentifier,
  },
  ModuleIsCleaned {
    module_identifier: ModuleIdentifier,
    dependent_module_identifiers: Vec<ModuleIdentifier>,
  },
}

impl CleanTask {
  pub fn run(self, compilation: &mut Compilation) -> CleanTaskResult {
    let module_identifier = self.module_identifier;
    let mgm = match compilation
      .module_graph
      .module_graph_module_by_identifier(&module_identifier)
    {
      Some(mgm) => mgm,
      None => {
        return CleanTaskResult::ModuleIsCleaned {
          module_identifier,
          dependent_module_identifiers: vec![],
        }
      }
    };

    if !mgm.incoming_connections.is_empty() {
      return CleanTaskResult::ModuleIsUsed { module_identifier };
    }

    let dependent_module_identifiers: Vec<ModuleIdentifier> = compilation
      .module_graph
      .get_module_all_depended_modules(&module_identifier)
      .expect("should have module")
      .into_iter()
      .copied()
      .collect();
    compilation.module_graph.revoke_module(&module_identifier);
    CleanTaskResult::ModuleIsCleaned {
      module_identifier,
      dependent_module_identifiers,
    }
  }
}

pub type CleanQueue = WorkerQueue<CleanTask>;

pub type ModuleCreationCallback = Box<dyn FnOnce(&BoxModule) + Send>;

pub type QueueHandleCallback = Box<dyn FnOnce(ModuleIdentifier, &mut Compilation) + Send + Sync>;

#[derive(Debug)]
pub enum QueueTask {
  Factorize(Box<FactorizeTask>),
  Add(Box<AddTask>),
  Build(Box<BuildTask>),
  ProcessDependencies(Box<ProcessDependenciesTask>),

  Subscription(Box<Subscription>),
}

#[derive(Debug, Copy, Clone)]
pub enum TaskCategory {
  Factorize = 0,
  Add = 1,
  Build = 2,
  ProcessDependencies = 3,
}

pub struct Subscription {
  category: TaskCategory,
  key: String,
  callback: QueueHandleCallback,
}

impl std::fmt::Debug for Subscription {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Subscription")
      .field("category", &self.category)
      .finish()
  }
}

#[derive(Clone)]
pub struct QueueHandler {
  sender: UnboundedSender<QueueTask>,
}

impl std::fmt::Debug for QueueHandler {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("QueueHandler")
      .field("sender", &self.sender)
      .finish()
  }
}

impl QueueHandler {
  pub fn wait_for(&self, key: String, category: TaskCategory, callback: QueueHandleCallback) {
    self
      .sender
      .send(QueueTask::Subscription(Box::new(Subscription {
        category,
        key,
        callback,
      })))
      .expect("failed to wait task");
  }
}

pub struct QueueHandlerProcessor {
  receiver: UnboundedReceiver<QueueTask>,
  callbacks: [FxHashMap<String, Vec<QueueHandleCallback>>; 4],
  finished: [FxHashMap<String, ModuleIdentifier>; 4],
}

impl QueueHandlerProcessor {
  pub fn try_process(
    &mut self,
    compilation: &mut Compilation,
    factorize_queue: &mut FactorizeQueue,
    add_queue: &mut AddQueue,
    build_queue: &mut BuildQueue,
    process_dependencies_queue: &mut ProcessDependenciesQueue,
  ) {
    while let Ok(task) = self.receiver.try_recv() {
      match task {
        QueueTask::Factorize(task) => {
          factorize_queue.add_task(*task);
        }
        QueueTask::Add(task) => {
          add_queue.add_task(*task);
        }
        QueueTask::Build(task) => {
          build_queue.add_task(*task);
        }
        QueueTask::ProcessDependencies(task) => {
          process_dependencies_queue.add_task(*task);
        }
        QueueTask::Subscription(subscription) => {
          let Subscription {
            category,
            key,
            callback,
          } = *subscription;
          if let Some(module) = self.finished[category as usize].get(&key) {
            // already finished
            callback(*module, compilation);
          } else {
            self.callbacks[category as usize]
              .entry(key)
              .or_default()
              .push(callback);
          }
        }
      }
    }
  }

  pub fn flush_callback(
    &mut self,
    category: TaskCategory,
    key: &str,
    module: ModuleIdentifier,
    compilation: &mut Compilation,
  ) {
    self.finished[category as usize].insert(key.into(), module);
    if let Some(callbacks) = self.callbacks[category as usize].get_mut(key) {
      while let Some(cb) = callbacks.pop() {
        cb(module, compilation);
      }
    }
  }
}

pub fn create_queue_handle() -> (QueueHandler, QueueHandlerProcessor) {
  let (tx, rx) = unbounded_channel();

  (
    QueueHandler { sender: tx },
    QueueHandlerProcessor {
      receiver: rx,
      callbacks: Default::default(),
      finished: Default::default(),
    },
  )
}

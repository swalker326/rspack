use std::sync::Arc;

pub use rspack_loader_runner::{run_loaders, Content, Loader, LoaderContext};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  BuildTimeExecutionTask, Compilation, CompilerOptions, Context, ModuleIdentifier, ResolverFactory,
};

#[derive(Debug, Clone)]
pub struct CompilerContext {
  pub options: Arc<CompilerOptions>,
  pub resolver_factory: Arc<ResolverFactory>,
  pub module: ModuleIdentifier,             // current module
  pub module_context: Option<Box<Context>>, // current module context
}

impl CompilerContext {
  pub async fn import_module(
    &self,
    request: String,
    public_path: Option<String>,
    base_uri: Option<String>,
  ) -> rspack_error::Result<String> {
    todo!()
  }
  //   if let Some(queue) = &self.buildtime_execution_queue {
  //     return Compilation::import_module_impl(
  //       request,
  //       public_path,
  //       base_uri,
  //       Some(self.module),
  //       self.module_context.clone(),
  //       queue.clone(),
  //     )
  //     .await;
  //   }

  //   Err(rspack_error::internal_error!(
  //     "Can not call import_module without buildtime_execution_queue"
  //   ))
  // }
}

pub type LoaderRunnerContext = CompilerContext;

pub type BoxLoader = Arc<dyn Loader<LoaderRunnerContext>>;

use napi::Either;
use napi_derive::napi;
use rspack_core::mf::{
  consume_shared_plugin::{ConsumeOptions, ConsumeVersion},
  container_plugin::{ContainerPluginOptions, ExposeOptions},
  container_reference_plugin::{ContainerReferencePluginOptions, RemoteOptions},
  provide_shared_plugin::{ProvideOptions, ProvideVersion},
};

use crate::RawLibraryOptions;

#[derive(Debug)]
#[napi(object)]
pub struct RawContainerPluginOptions {
  pub name: String,
  pub share_scope: String,
  pub library: RawLibraryOptions,
  pub runtime: Option<String>,
  pub filename: Option<String>,
  pub exposes: Vec<RawExposeOptions>,
}

impl From<RawContainerPluginOptions> for ContainerPluginOptions {
  fn from(value: RawContainerPluginOptions) -> Self {
    Self {
      name: value.name,
      share_scope: value.share_scope,
      library: value.library.into(),
      runtime: value.runtime,
      filename: value.filename.map(|f| f.into()),
      exposes: value.exposes.into_iter().map(|e| e.into()).collect(),
    }
  }
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct RawExposeOptions {
  pub key: String,
  pub name: Option<String>,
  pub import: Vec<String>,
}

impl From<RawExposeOptions> for (String, ExposeOptions) {
  fn from(value: RawExposeOptions) -> Self {
    (
      value.key,
      ExposeOptions {
        name: value.name,
        import: value.import,
      },
    )
  }
}

#[derive(Debug)]
#[napi(object)]
pub struct RawContainerReferencePluginOptions {
  pub remote_type: String,
  pub remotes: Vec<RawRemoteOptions>,
  pub share_scope: Option<String>,
}

impl From<RawContainerReferencePluginOptions> for ContainerReferencePluginOptions {
  fn from(value: RawContainerReferencePluginOptions) -> Self {
    Self {
      remote_type: value.remote_type,
      remotes: value.remotes.into_iter().map(|e| e.into()).collect(),
      share_scope: value.share_scope,
    }
  }
}

#[derive(Debug)]
#[napi(object)]
pub struct RawRemoteOptions {
  pub key: String,
  pub external: Vec<String>,
  pub share_scope: String,
}

impl From<RawRemoteOptions> for (String, RemoteOptions) {
  fn from(value: RawRemoteOptions) -> Self {
    (
      value.key,
      RemoteOptions {
        external: value.external,
        share_scope: value.share_scope,
      },
    )
  }
}

#[derive(Debug)]
#[napi(object)]
pub struct RawProvideOptions {
  pub key: String,
  pub share_key: String,
  pub share_scope: String,
  #[napi(ts_type = "string | false | undefined")]
<<<<<<< HEAD
  pub version: Option<RawVersion>,
=======
  pub version: Option<RawProvideVersion>,
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
  pub eager: bool,
}

impl From<RawProvideOptions> for (String, ProvideOptions) {
  fn from(value: RawProvideOptions) -> Self {
    (
      value.key,
      ProvideOptions {
        share_key: value.share_key,
        share_scope: value.share_scope,
<<<<<<< HEAD
        version: value.version.map(|v| RawVersionWrapper(v).into()),
=======
        version: value.version.map(|v| RawProvideVersionWrapper(v).into()),
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
        eager: value.eager,
      },
    )
  }
}

<<<<<<< HEAD
#[derive(Debug)]
#[napi(object)]
pub struct RawConsumeOptions {
  pub key: String,
  pub import: Option<String>,
  pub import_resolved: Option<String>,
  pub share_key: String,
  pub share_scope: String,
  #[napi(ts_type = "string | false | undefined")]
  pub required_version: Option<RawVersion>,
  pub package_name: Option<String>,
  pub strict_version: bool,
  pub singleton: bool,
  pub eager: bool,
}

impl From<RawConsumeOptions> for (String, ConsumeOptions) {
  fn from(value: RawConsumeOptions) -> Self {
    (
      value.key,
      ConsumeOptions {
        import: value.import,
        import_resolved: value.import_resolved,
        share_key: value.share_key,
        share_scope: value.share_scope,
        required_version: value.required_version.map(|v| RawVersionWrapper(v).into()),
        package_name: value.package_name,
        strict_version: value.strict_version,
        singleton: value.singleton,
        eager: value.eager,
      },
    )
  }
}

pub type RawVersion = Either<String, bool>;

struct RawVersionWrapper(RawVersion);

impl From<RawVersionWrapper> for ProvideVersion {
  fn from(value: RawVersionWrapper) -> Self {
=======
pub type RawProvideVersion = Either<String, bool>;

struct RawProvideVersionWrapper(RawProvideVersion);

impl From<RawProvideVersionWrapper> for ProvideVersion {
  fn from(value: RawProvideVersionWrapper) -> Self {
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))
    match value.0 {
      Either::A(s) => ProvideVersion::Version(s),
      Either::B(_) => ProvideVersion::False,
    }
  }
}
<<<<<<< HEAD

impl From<RawVersionWrapper> for ConsumeVersion {
  fn from(value: RawVersionWrapper) -> Self {
    match value.0 {
      Either::A(s) => ConsumeVersion::Version(s),
      Either::B(_) => ConsumeVersion::False,
    }
  }
}
=======
>>>>>>> 24ebe5e86 (feat: Module Federation, part 3, ProvideSharedPlugin (#4778))

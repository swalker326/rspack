use rspack_core::Target;

use crate::RawOptionsApply;

pub type RawTarget = Vec<String>;

impl RawOptionsApply for RawTarget {
  type Options = Target;

  fn apply(
    self,
    _: &mut Vec<rspack_core::BoxPlugin>,
  ) -> Result<Self::Options, rspack_error::Error> {
    Target::new(&self)
  }
}

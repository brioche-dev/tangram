use super::Module;
use crate::{error::Result, instance::Instance};

impl Module {
	pub async fn version(&self, tg: &Instance) -> Result<i32> {
		match self {
			Module::Library(_) | Module::Normal { .. } => Ok(0),
			#[cfg(feature = "language")]
			Module::Document(document) => document.version(tg).await,
		}
	}
}

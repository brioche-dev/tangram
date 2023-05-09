pub use self::{instance::Instance, metadata::Metadata, specifier::Specifier};
use crate::{artifact::Artifact, error::Result};
use std::{path::PathBuf, sync::Arc};

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

mod analyze;
pub mod checkin;
pub mod dependency;
pub mod instance;
mod instantiate;
pub mod metadata;
pub mod specifier;

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Package {
	artifact: Artifact,
	path: Option<PathBuf>,
}

impl Package {
	#[must_use]
	pub fn new(artifact: Artifact, path: Option<PathBuf>) -> Self {
		Self { artifact, path }
	}

	pub async fn with_specifier(
		tg: &Arc<crate::instance::Instance>,
		specifier: Specifier,
	) -> Result<Self> {
		match specifier {
			Specifier::Path(path) => Ok(Self::check_in(tg, &path).await?),
			Specifier::Registry(_) => todo!(),
		}
	}

	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}
}

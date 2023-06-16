use super::Symlink;
use crate::{artifact::Artifact, error::Result, instance::Instance};

impl Symlink {
	pub async fn resolve(&self, tg: &Instance) -> Result<Option<Artifact>> {
		self.resolve_from(tg, None).await
	}

	#[allow(clippy::unused_async)]
	pub async fn resolve_from(
		&self,
		_tg: &Instance,
		_from: Option<&Symlink>,
	) -> Result<Option<Artifact>> {
		unimplemented!()
	}
}

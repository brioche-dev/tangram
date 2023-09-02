use crate as tg;
use crate::error::Result;
use crate::instance::Instance;

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Symlink {
	#[tangram_serialize(id = 0)]
	target: tg::Template,
}

crate::value!(Symlink);

impl tg::Symlink {
	#[must_use]
	pub fn new(target: tg::Template) -> Self {
		Symlink { target }.into()
	}

	pub async fn target(&self, tg: &Instance) -> Result<tg::Template> {
		Ok(self.get(tg).await?.target.clone())
	}
}

impl Symlink {
	#[must_use]
	pub fn children(&self) -> Vec<tg::Value> {
		vec![self.target.clone().into()]
	}
}

// use super::Symlink;
// use crate::{artifact::Artifact, error::Result, instance::Instance};

// impl Symlink {
// 	pub async fn resolve(&self, tg: &Instance) -> Result<Option<Artifact>> {
// 		self.resolve_from(tg, None).await
// 	}

// 	#[allow(clippy::unused_async)]
// 	pub async fn resolve_from(
// 		&self,
// 		_tg: &Instance,
// 		_from: Option<&Symlink>,
// 	) -> Result<Option<Artifact>> {
// 		unimplemented!()
// 	}
// }

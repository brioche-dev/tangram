use super::{Identifier, Specifier};
use crate::{
	error::{return_error, Result},
	Instance,
};

impl Instance {
	#[allow(clippy::unused_async)]
	pub async fn resolve_package(
		&self,
		specifier: &Specifier,
		referrer: Option<&Identifier>,
	) -> Result<Identifier> {
		match specifier {
			Specifier::Path(specifier_path) => match referrer {
				Some(Identifier::Path(referrer_path)) => {
					let path = referrer_path.join(specifier_path);
					let path = tokio::fs::canonicalize(&path).await?;
					Ok(Identifier::Path(path))
				},

				Some(Identifier::Hash(_)) => {
					return_error!("Cannot resolve a path specifier relative to a hash referrer.")
				},

				None => Ok(Identifier::Path(specifier_path.clone())),
			},

			Specifier::Registry(_) => todo!(),
		}
	}
}

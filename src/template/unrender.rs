use super::Template;
use crate::{artifact, Cli};

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn unrender(_string: String, _artifact_hashes: Vec<artifact::Hash>) -> Template {
		todo!()
	}
}

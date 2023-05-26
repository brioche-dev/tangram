use super::Artifact;
use crate::{
	checksum::{self, Checksum},
	error::Result,
	instance::Instance,
};

impl Artifact {
	#[allow(clippy::unused_async)]
	pub async fn checksum(
		&self,
		_tg: &Instance,
		_algorithm: checksum::Algorithm,
	) -> Result<Checksum> {
		unimplemented!()
	}
}

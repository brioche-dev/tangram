use crate::{artifact, error::Result, util::fs};

pub struct Client;

impl Client {
	pub async fn new() -> Result<Client> {
		todo!()
	}

	pub fn checkin(&self, path: &fs::Path) -> Result<artifact::Hash> {
		todo!()
	}

	pub async fn checkout(&self, hash: artifact::Hash) -> Result<fs::PathBuf> {
		todo!()
	}
}

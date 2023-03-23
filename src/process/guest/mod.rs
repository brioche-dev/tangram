pub struct Client;

impl Client {
	pub async fn new() -> Result<Client> {
		todo!()
	}

	pub fn checkin(&self, path: &Path) -> Result<Hash> {
		todo!()
	}

	//
	pub async fn checkout(&self, hash: Hash) -> Result<PathBuf> {
		todo!()
	}
}

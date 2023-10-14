use super::Server;
use tangram_client::{return_error, Result};

impl Server {
	pub async fn clean(&self) -> Result<()> {
		return_error!("This is not yet implemented.");
	}
}

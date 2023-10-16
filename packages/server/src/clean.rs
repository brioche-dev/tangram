use super::Server;
use tangram_client as tg;
use tg::{return_error, Result};

impl Server {
	pub async fn clean(&self) -> Result<()> {
		return_error!("This is not yet implemented.");
	}
}

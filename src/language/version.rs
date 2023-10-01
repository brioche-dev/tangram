use super::Module;
use crate::{error::Result, server::Server};

impl Module {
	pub async fn version(&self, server: &Server) -> Result<i32> {
		match self {
			Module::Library(_) | Module::Normal { .. } => Ok(0),
			Module::Document(document) => document.version(server).await,
		}
	}
}

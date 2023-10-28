use crate::nfs::{
	server::{Context, Server},
	types::*,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_sec_info(&self, ctx: &Context, arg: &str) -> i32 {
		let Some(parent) = ctx.current_file_handle else {
			return NFS4ERR_NOFILEHANDLE;
		};
		match self.lookup(parent, arg).await {
			Ok(_) => NFS4_OK,
			Err(e) => e,
		}
	}
}

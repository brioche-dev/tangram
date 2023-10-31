use crate::nfs::{
	types::{nfsstat4, SECINFO4args, SECINFO4res},
	Context, Server,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_sec_info(&self, ctx: &Context, arg: SECINFO4args) -> SECINFO4res {
		let Some(parent) = ctx.current_file_handle else {
			return SECINFO4res::Default(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};
		let Ok(name) = std::str::from_utf8(&arg.name) else {
			return SECINFO4res::Default(nfsstat4::NFS4ERR_NOENT);
		};
		match self.lookup(parent, name).await {
			Ok(_) => SECINFO4res::NFS4_OK(vec![]),
			Err(e) => SECINFO4res::Default(e),
		}
	}
}

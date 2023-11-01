use crate::nfs::{
	state::NodeKind,
	types::{dirlist4, entry4, nfsstat4, READDIR4args, READDIR4res, READDIR4resok},
	Context, Server,
};
use num::ToPrimitive;

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_readdir(&self, ctx: &Context, arg: READDIR4args) -> READDIR4res {
		let Some(fh) = ctx.current_file_handle else {
			return READDIR4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh).await else {
			return READDIR4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		let cookie = arg.cookie.to_usize().unwrap();
		let mut count = 0;

		let entries = match &node.kind {
			NodeKind::Directory { directory, .. } => {
				let Ok(entries) = directory.entries(self.client.as_ref()).await else {
					return READDIR4res::Error(nfsstat4::NFS4ERR_IO);
				};
				entries.clone()
			},
			NodeKind::Root { .. } => Default::default(),
			_ => return READDIR4res::Error(nfsstat4::NFS4ERR_NOTDIR),
		};

		let mut reply = Vec::with_capacity(entries.len());
		let names = entries.keys().map(AsRef::as_ref);

		let mut eof = true;
		for (cookie, name) in [".", ".."]
			.into_iter()
			.chain(names)
			.enumerate()
			.skip(cookie)
		{
			let node = match name {
				"." => node.clone(),
				".." => node.parent.upgrade().unwrap(),
				_ => match self.get_or_create_child_node(node.clone(), name).await {
					Ok(node) => node,
					Err(e) => return READDIR4res::Error(e),
				},
			};
			let attrs = self
				.get_attr(node.id, arg.attr_request.clone())
				.await
				.unwrap();
			let cookie = cookie.to_u64().unwrap();
			let name = name.to_owned();

			// Size of the cookie + size of the attr + size of the name
			count += std::mem::size_of_val(&cookie); // u64
			count += 4 + 4 * attrs.attrmask.0.len(); // bitmap4
			count += 4 + attrs.attr_vals.len(); // opaque<>
			count += 4 + name.len(); // utf8_cstr

			if count > arg.dircount.to_usize().unwrap() {
				eof = false;
				break;
			}

			let name = name.as_bytes().into();
			reply.push(entry4 {
				cookie,
				name,
				attrs,
			});
		}

		let cookieverf = fh.to_be_bytes();
		let reply = dirlist4 {
			entries: reply,
			eof,
		};
		READDIR4res::NFS4_OK(READDIR4resok { cookieverf, reply })
	}
}

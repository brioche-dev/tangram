use num::ToPrimitive;

use crate::vfs::nfs::{
	server::{Context, Server},
	state::NodeKind,
	types::*,
	xdr::{FromXdr, ToXdr},
};

#[derive(Debug, Clone)]
pub struct Arg {
	pub cookie: Cookie,
	pub cookie_verf: [u8; NFS4_VERIFIER_SIZE],
	pub dircount: Count,
	pub maxcount: Count,
	pub attr_request: Bitmap,
}

#[derive(Debug, Clone)]
pub enum ResOp {
	Ok {
		cookieverf: [u8; NFS4_VERIFIER_SIZE],
		reply: Vec<Entry>,
		eof: bool,
	},
	Err(i32),
}

const SKIP: usize = 2;

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_readdir(&self, ctx: &Context, arg: Arg) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return ResOp::Err(NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh.node).await else {
			return ResOp::Err(NFS4ERR_BADHANDLE);
		};

		let cookie = arg.cookie.to_usize().unwrap();
		let mut count = 0;

		let entries = match &node.kind {
			NodeKind::Directory { directory, .. } => {
				let Ok(entries) = directory.entries(&self.client).await else {
					return ResOp::Err(NFS4ERR_IO);
				};
				entries.clone()
			},
			NodeKind::Root { .. } => Default::default(),
			_ => return ResOp::Err(NFS4ERR_NOTDIR),
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
					Err(e) => return ResOp::Err(e),
				},
			};
			let attrs = self
				.get_attr(FileHandle { node: node.id }, arg.attr_request.clone())
				.await
				.unwrap();
			let cookie = cookie.to_u64().unwrap();
			let name = name.to_owned();

			// Size of the cookie + size of the attr + size of the name
			count += std::mem::size_of_val(&cookie); // u64
			count += 4 + 4 * attrs.attr_mask.0.len(); // bitmap4
			count += 4 + attrs.attr_vals.len(); // opaque<>
			count += 4 + name.len(); // utf8_cstr

			if count > arg.dircount.to_usize().unwrap() {
				eof = false;
				break;
			}

			reply.push(Entry {
				cookie,
				name,
				attrs,
			});
		}

		let cookieverf = fh.node.to_be_bytes();
		ResOp::Ok {
			cookieverf,
			reply,
			eof,
		}
	}
}
impl FromXdr for Arg {
	fn decode(
		decoder: &mut crate::vfs::nfs::xdr::Decoder<'_>,
	) -> Result<Self, crate::vfs::nfs::xdr::Error> {
		let cookie = decoder.decode()?;
		let cookie_verf = decoder.decode_n()?;
		let dircount = decoder.decode()?;
		let maxcount = decoder.decode()?;
		let attr_request = decoder.decode()?;
		Ok(Self {
			cookie,
			cookie_verf,
			dircount,
			maxcount,
			attr_request,
		})
	}
}

impl ResOp {
	pub fn status(&self) -> i32 {
		match self {
			Self::Ok { .. } => NFS4_OK,
			Self::Err(e) => *e,
		}
	}
}

impl ToXdr for ResOp {
	fn encode<W>(
		&self,
		encoder: &mut crate::vfs::nfs::xdr::Encoder<W>,
	) -> Result<(), crate::vfs::nfs::xdr::Error>
	where
		W: std::io::Write,
	{
		match self {
			Self::Ok {
				cookieverf,
				reply,
				eof,
			} => {
				encoder.encode_int(NFS4_OK)?;
				encoder.encode_n(*cookieverf)?;
				for entry in reply {
					encoder.encode_bool(true)?;
					encoder.encode(entry)?;
				}
				encoder.encode_bool(false)?;
				encoder.encode(eof)?;
			},
			Self::Err(e) => {
				encoder.encode(e)?;
			},
		}
		Ok(())
	}
}

use super::{ops::*, types::*, xdr};

#[derive(Debug, Clone)]
pub struct CompoundArgs {
	pub tag: String,
	pub minor_version: u32,
	pub args: Vec<Arg>,
}

#[derive(Debug, Clone)]
pub struct CompoundReply {
	pub status: i32,
	pub tag: String,
	pub results: Vec<ResultOp>,
}

#[derive(Debug, Clone)]
pub enum Arg {
	Access(access::Arg),
	Close(close::Arg),
	GetAttr(Bitmap),
	GetFileHandle,
	Lookup(lookup::Arg),
	Open(open::Arg),
	PutFileHandle(FileHandle),
	PutRootFileHandle,
	Read(read::Arg),
	ReadDir(readdir::Arg),
	ReadLink,
	Renew(set_client_id::ClientId),
	RestoreFileHandle,
	SaveFileHandle,
	SecInfo(String),
	SetClientId(set_client_id::Arg),
	SetClientIdConfirm(set_client_id_confirm::Arg),
	Unsupported(i32),
	Illegal,
}

#[derive(Debug, Clone)]
pub enum ResultOp {
	Access(access::ResOp),
	Close(close::ResOp),
	GetAttr(getattr::ResOp),
	LookupResult(lookup::ResOp),
	OpenResult(open::ResOp),
	PutFileHandle(i32),
	GetFileHandle(Result<FileHandle, i32>),
	PutRootFileHandle(i32),
	Read(read::ResOp),
	ReadDir(readdir::ResOp),
	ReadLink(readlink::ResOp),
	Renew(i32),
	RestoreFileHandle(i32),
	SaveFileHandle(i32),
	SecInfo(i32),
	SetClientId(set_client_id::ResOp),
	SetClientIdConfirm(i32),
	Unsupported(i32),
	Illegal,
}

impl Arg {
	pub fn opcode(&self) -> i32 {
		match self {
			Self::Access(_) => 3,
			Self::Close(_) => 4,
			Self::GetAttr(_) => 9,
			Self::GetFileHandle => 10,
			Self::Lookup(_) => 15,
			Self::Open(_) => 18,
			Self::PutFileHandle(_) => 22,
			Self::PutRootFileHandle => 24,
			Self::Read(_) => 25,
			Self::ReadDir(_) => 26,
			Self::ReadLink => 27,
			Self::Renew(_) => 30,
			Self::RestoreFileHandle => 31,
			Self::SaveFileHandle => 32,
			Self::SecInfo(_) => 33,
			Self::SetClientId(_) => 35,
			Self::SetClientIdConfirm(_) => 36,
			Self::Unsupported(op) => *op,
			Self::Illegal => NFS4ERR_OP_ILLEGAL,
		}
	}
}

impl xdr::FromXdr for Arg {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let opcode = decoder.decode_int()?;
		match opcode {
			3 => Ok(Arg::Access(decoder.decode()?)),
			4 => Ok(Arg::Close(decoder.decode()?)),
			9 => Ok(Arg::GetAttr(decoder.decode()?)),
			10 => Ok(Arg::GetFileHandle),
			15 => Ok(Arg::Lookup(decoder.decode()?)),
			18 => Ok(Arg::Open(decoder.decode()?)),
			22 => Ok(Arg::PutFileHandle(decoder.decode()?)),
			24 => Ok(Arg::PutRootFileHandle),
			25 => Ok(Arg::Read(decoder.decode()?)),
			26 => Ok(Arg::ReadDir(decoder.decode()?)),
			27 => Ok(Arg::ReadLink),
			30 => Ok(Arg::Renew(decoder.decode()?)),
			31 => Ok(Arg::RestoreFileHandle),
			32 => Ok(Arg::SaveFileHandle),
			33 => Ok(Arg::SecInfo(decoder.decode()?)),
			35 => Ok(Arg::SetClientId(decoder.decode()?)),
			36 => Ok(Arg::SetClientIdConfirm(decoder.decode()?)),
			opcode => {
				if (3..=56).contains(&opcode) {
					tracing::warn!(?opcode, "Unsupported opcode.");
					Ok(Arg::Unsupported(opcode))
				} else {
					tracing::warn!(?opcode, "Illegal opcode.");
					Ok(Arg::Illegal)
				}
			},
		}
	}
}

impl xdr::FromXdr for CompoundArgs {
	fn decode(decoder: &mut super::xdr::Decoder<'_>) -> Result<Self, super::xdr::Error> {
		let tag = std::str::from_utf8(decoder.decode_opaque()?)?;
		let minor_version = decoder.decode_uint()?;
		let args = decoder.decode()?;
		Ok(Self {
			tag: tag.to_owned(),
			minor_version,
			args,
		})
	}
}

impl xdr::ToXdr for CompoundReply {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_int(self.status)?;
		encoder.encode_opaque(self.tag.as_bytes())?;
		encoder.encode(&self.results)?;
		Ok(())
	}
}

impl ResultOp {
	pub fn opcode(&self) -> i32 {
		match self {
			Self::Access(_) => 3,
			Self::Close(_) => 4,
			Self::GetAttr(_) => 9,
			Self::GetFileHandle(_) => 10,
			Self::LookupResult(_) => 15,
			Self::OpenResult(_) => 18,
			Self::PutFileHandle(_) => 22,
			Self::PutRootFileHandle(_) => 24,
			Self::Read(_) => 25,
			Self::ReadDir(_) => 26,
			Self::ReadLink(_) => 27,
			Self::Renew(_) => 30,
			Self::RestoreFileHandle(_) => 31,
			Self::SaveFileHandle(_) => 32,
			Self::SecInfo(_) => 33,
			Self::SetClientId(_) => 35,
			Self::SetClientIdConfirm(_) => 36,
			Self::Unsupported(op) => *op,
			Self::Illegal => NFS4ERR_OP_ILLEGAL,
		}
	}

	pub fn status(&self) -> i32 {
		match self {
			Self::Access(res) => res.status(),
			Self::Close(close) => close.status(),
			Self::GetAttr(res) => res.status(),
			Self::GetFileHandle(_) => NFS4_OK,
			Self::LookupResult(status) => *status,
			Self::OpenResult(res) => res.status(),
			Self::PutFileHandle(status) => *status,
			Self::PutRootFileHandle(status) => *status,
			Self::Read(res) => res.status(),
			Self::ReadDir(res) => res.status(),
			Self::ReadLink(res) => res.status(),
			Self::Renew(status) => *status,
			Self::RestoreFileHandle(status) => *status,
			Self::SaveFileHandle(status) => *status,
			Self::SecInfo(status) => *status,
			Self::SetClientId(res) => res.status(),
			Self::SetClientIdConfirm(status) => *status,
			Self::Unsupported(_) => NFS4ERR_IO,
			Self::Illegal => NFS4ERR_OP_ILLEGAL,
		}
	}
}

impl xdr::ToXdr for ResultOp {
	fn encode<W>(&self, encoder: &mut super::xdr::Encoder<W>) -> Result<(), super::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_int(self.opcode())?;
		if self.status() != 0 {
			tracing::warn!(?self, "Returning Error.");
		}
		match self {
			Self::Access(res) => encoder.encode(res)?,
			Self::Close(res) => encoder.encode(res)?,
			Self::GetAttr(res) => encoder.encode(res)?,
			Self::OpenResult(res) => encoder.encode(res)?,
			Self::GetFileHandle(res) => match res {
				Ok(fh) => {
					encoder.encode_int(0)?;
					encoder.encode(fh)?;
				},
				Err(e) => {
					encoder.encode_int(*e)?;
				},
			},
			Self::Read(res) => encoder.encode(res)?,
			Self::ReadDir(res) => encoder.encode(res)?,
			Self::ReadLink(res) => encoder.encode(res)?,
			Self::SetClientId(res) => encoder.encode(res)?,
			Self::SecInfo(res) => {
				encoder.encode_int(0i32)?;
				encoder.encode(res)?;
			},
			_ => encoder.encode_int(self.status())?,
		}
		Ok(())
	}
}

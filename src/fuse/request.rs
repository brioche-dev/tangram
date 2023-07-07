use std::os::unix::prelude::OsStrExt;

use super::abi;
use zerocopy::FromBytes;

#[derive(Clone, Debug)]
pub struct Request {
	pub header: abi::fuse_in_header,
	pub arg: Arg,
}

fn read<T>(bytes: &[u8]) -> Option<T>
where
	T: FromBytes,
{
	T::read_from_prefix(bytes)
}

fn read_string(bytes: &[u8]) -> Option<std::ffi::OsString> {
	if *bytes.last()? != 0 {
		return None;
	}

	let s = std::ffi::OsStr::from_bytes(&bytes[..bytes.len() - 1]).to_owned();
	Some(s)
}

impl Request {
	// Deserialize a request from raw bytes.
	pub fn deserialize(data: &[u8]) -> Option<Self> {
		let header: abi::fuse_in_header = read(data)?;

		let header_len = std::mem::size_of::<abi::fuse_in_header>();
		let data_len = data.len();
		if data_len < header.len as usize {
			tracing::error!(?data_len, ?header, "Not enough data for FUSE request.");
			return None;
		}

		let data = &data[header_len..];
		let opcode = header
			.opcode
			.try_into()
			.map_err(|e| tracing::error!(?e, ?header))
			.ok()?;

		let arg = match opcode {
			abi::fuse_opcode::FUSE_INIT => Arg::Initialize(read(data)?),
			abi::fuse_opcode::FUSE_DESTROY => Arg::Destroy,
			abi::fuse_opcode::FUSE_LOOKUP => Arg::Lookup(read_string(data)?),
			abi::fuse_opcode::FUSE_GETATTR => Arg::GetAttr,
			abi::fuse_opcode::FUSE_READLINK => Arg::ReadLink,
			abi::fuse_opcode::FUSE_OPEN => Arg::Open(read(data)?),
			abi::fuse_opcode::FUSE_READ => Arg::Read(read(data)?),
			abi::fuse_opcode::FUSE_RELEASE => Arg::Release,
			abi::fuse_opcode::FUSE_OPENDIR => Arg::OpenDir(read(data)?),
			abi::fuse_opcode::FUSE_READDIR => Arg::ReadDir(read(data)?),
			abi::fuse_opcode::FUSE_READDIRPLUS => Arg::ReadDirPlus(read(data)?),
			abi::fuse_opcode::FUSE_RELEASEDIR => Arg::ReleaseDir,
			abi::fuse_opcode::FUSE_FLUSH => Arg::Flush(read(data)?),
			_ => Arg::Unsupported(opcode),
		};

		Some(Self { header, arg })
	}
}

#[derive(Clone, Debug)]
pub enum Arg {
	Initialize(abi::fuse_init_in),
	Destroy,
	Lookup(std::ffi::OsString),
	GetAttr,
	ReadLink,
	Open(abi::fuse_open_in),
	Read(abi::fuse_read_in),
	Release,
	OpenDir(abi::fuse_open_in),
	ReadDir(abi::fuse_read_in),
	ReadDirPlus(abi::fuse_read_in),
	ReleaseDir,
	Flush(abi::fuse_flush_in),
	Unsupported(abi::fuse_opcode),
}

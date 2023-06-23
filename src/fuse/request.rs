use super::abi;
use super::argument::ArgumentIterator;
use crate::{
	error::{error, Result},
	return_error,
};

impl<'a> Request<'a> {
	// Deserialize a request from raw bytes.
	pub fn deserialize(data: &'a [u8]) -> Result<Self> {
		let data_len = data.len();
		let mut arg_iter = ArgumentIterator::new(data);

		let header: &abi::fuse_in_header = arg_iter
			.fetch()
			.ok_or(error!("Failed to deserialize header length."))?;

		if data_len < header.len as usize {
			return_error!("Failed to deserialize FUSE request header.");
		}

		let data = &data[std::mem::size_of::<abi::fuse_in_header>()..header.len as usize];
		let mut data = ArgumentIterator::new(data);
		let data = match header.opcode.try_into().unwrap() {
			abi::fuse_opcode::FUSE_INIT => Some(RequestData::Initialize(Initialize {
				data: data.fetch().unwrap(),
			})),
			abi::fuse_opcode::FUSE_DESTROY => Some(RequestData::Destroy),
			abi::fuse_opcode::FUSE_LOOKUP => Some(RequestData::Lookup(Lookup {
				name: data.fetch_str().unwrap(),
			})),
			abi::fuse_opcode::FUSE_GETATTR => Some(RequestData::GetAttr),
			abi::fuse_opcode::FUSE_READLINK => Some(RequestData::ReadLink),
			abi::fuse_opcode::FUSE_OPEN => Some(RequestData::Open(Open {
				data: data.fetch().unwrap(),
			})),
			abi::fuse_opcode::FUSE_READ => Some(RequestData::Read(Read {
				data: data.fetch().unwrap(),
			})),
			abi::fuse_opcode::FUSE_OPENDIR => Some(RequestData::OpenDir(OpenDir {
				data: data.fetch().unwrap(),
			})),
			abi::fuse_opcode::FUSE_READDIR => Some(RequestData::ReadDir(ReadDir {
				data: data.fetch().unwrap(),
			})),
			abi::fuse_opcode::FUSE_ACCESS => Some(RequestData::Access(Access {
				data: data.fetch().unwrap(),
			})),
			abi::fuse_opcode::FUSE_STATFS => Some(RequestData::StatFs),
			_ => return_error!("Unsupported FUSE opcode: {}.", header.opcode),
		};

		let data = data.ok_or(error!("Failed to parse request."))?;
		Ok(Self { header, data })
	}
}

#[derive(Copy, Clone, Debug)]
pub struct Request<'a> {
	pub header: &'a abi::fuse_in_header,
	pub data: RequestData<'a>,
}

#[derive(Copy, Clone, Debug)]
pub enum RequestData<'a> {
	Initialize(Initialize<'a>),
	Destroy,
	Lookup(Lookup<'a>),
	GetAttr,
	ReadLink,
	Open(Open<'a>),
	Read(Read<'a>),
	OpenDir(OpenDir<'a>),
	ReadDir(ReadDir<'a>),
	Access(Access<'a>),
	StatFs,
}

#[derive(Copy, Clone, Debug)]
pub struct Initialize<'a> {
	data: &'a abi::fuse_init_in,
}

#[derive(Copy, Clone, Debug)]
pub struct Lookup<'a> {
	name: &'a std::ffi::OsStr,
}

#[derive(Copy, Clone, Debug)]
pub struct Open<'a> {
	data: &'a abi::fuse_open_in,
}

#[derive(Copy, Clone, Debug)]
pub struct Read<'a> {
	data: &'a abi::fuse_read_in,
}

#[derive(Copy, Clone, Debug)]
pub struct OpenDir<'a> {
	data: &'a abi::fuse_open_in,
}

#[derive(Copy, Clone, Debug)]
pub struct ReadDir<'a> {
	data: &'a abi::fuse_read_in,
}

#[derive(Copy, Clone, Debug)]
pub struct Access<'a> {
	data: &'a abi::fuse_access_in,
}

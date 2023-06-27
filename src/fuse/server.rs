use super::{abi, request, response};
use zerocopy::AsBytes;

#[tracing::instrument(skip(_request))]
pub async fn initialize(
	_request: request::Request<'_>,
	arg: request::Initialize<'_>,
) -> response::Response {
	let response = abi::fuse_init_out {
		major: 7,                              // Major version that we support.
		minor: 9,                              // Minor version that we target.
		max_readahead: arg.data.max_readahead, // Reuse from the argument.
		max_write: 4096,                       // This is a limit on the size of messages.
		flags: abi::consts::FUSE_ASYNC_READ,   // Equivalent to no flags.
		unused: 0,                             // Padding.
	};

	response::Response::data(response.as_bytes())
}

#[tracing::instrument(skip(_request))]
pub async fn destroy(_request: request::Request<'_>) {}

#[tracing::instrument]
pub async fn lookup(request: request::Request<'_>, arg: request::Lookup<'_>) -> response::Response {
	let parent_inode = request.header.nodeid;
	if parent_inode == 1 && arg.name == "file.txt" {
		response::Response::entry(
			2,             // inode
			FILE_TXT_SIZE, // file_size
			1,             // num_blocks
			response::FileKind::File {
				is_executable: false,
			},
		)
	} else {
		response::Response::error(libc::ENOENT)
	}
}

#[tracing::instrument]
pub async fn getattr(request: request::Request<'_>) -> response::Response {
	// This represents how much time the kernel is allowed to cache the results of this function before it must re-request data.
	// TODO: since the file system is immutable and read-only (from the perspective of processes that acces it), this value could be "never." We should identify what that value is.
	let ttl: std::time::Duration = std::time::Duration::from_micros(100);

	let attr = abi::fuse_attr {
		ino: request.header.nodeid,
		size: 0,
		blocks: 0,
		atime: 0,
		mtime: 0,
		ctime: 0,
		atimensec: 0,
		mtimensec: 0,
		ctimensec: 0,
		nlink: 2, // number of hard links.
		mode: response::FileKind::Directory.mode(),
		uid: 1000,
		gid: 1000,
		rdev: 0,
		blksize: 512,
		padding: 0,
	};

	let response = abi::fuse_attr_out {
		attr_valid: ttl.as_secs(),
		attr_valid_nsec: ttl.subsec_nanos(),
		dummy: 0,
		attr,
	};

	response::Response::data(response.as_bytes())
}

#[tracing::instrument]
pub async fn readlink(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn open(request: request::Request<'_>, _arg: request::Open<'_>) -> response::Response {
	// Create the response object.
	let response = abi::fuse_open_out {
		fh: 0,         // No file handle.
		open_flags: 0, // No flags.
		padding: 0,    // Padding.
	};

	response::Response::data(response.as_bytes())
}

#[tracing::instrument]
pub async fn read(request: request::Request<'_>, arg: request::Read<'_>) -> response::Response {
	if request.header.nodeid == 2 {
		let file_contents = FILE_TXT_CONTENTS;
		let offset: usize = arg.data.offset.try_into().unwrap();
		let length = arg.data.size as usize;

		let range = offset..length.min(file_contents.len());
		let read_output = &file_contents[range];

		response::Response::data(read_output)
	} else {
		response::Response::error(libc::ENOENT)
	}
}

#[tracing::instrument]
pub async fn opendir(
	request: request::Request<'_>,
	_arg: request::OpenDir<'_>,
) -> response::Response {
	// Note:
	// - This must be made stateful (returning a valid file handle) or else we cannot correctly implement directory streams (POSIX opendir)
	let response = abi::fuse_open_out {
		fh: 0,         // No file handle.
		open_flags: 0, // No flags.
		padding: 0,    // Padding.
	};

	response::Response::data(response.as_bytes())
}

#[tracing::instrument]
pub async fn readdir(
	request: request::Request<'_>,
	_arg: request::ReadDir<'_>,
) -> response::Response {
	if request.header.nodeid != 1 {
		return response::Response::error(libc::ENOENT);
	}

	let entries = [
		response::DirectoryEntry {
			inode: 1,
			name: ".",
			kind: response::FileKind::Directory,
		},
		response::DirectoryEntry {
			inode: 1,
			name: "..",
			kind: response::FileKind::Directory,
		},
		response::DirectoryEntry {
			inode: 2,
			name: "file.txt",
			kind: response::FileKind::File {
				is_executable: false,
			},
		},
	];
	response::Response::directory(&entries)
}

#[tracing::instrument]
pub async fn access(
	request: request::Request<'_>,
	_arg: request::Access<'_>,
) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn statfs(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

/// release() is called on close() after flush().
#[tracing::instrument]
pub async fn release(_request: request::Request<'_>) -> response::Response {
	response::Response::error(0)
}

/// flush() is called on close() before release().
#[tracing::instrument]
pub async fn flush(_request: request::Request<'_>, _arg: request::Flush<'_>) -> response::Response {
	response::Response::error(0)
}

const FILE_TXT_CONTENTS: &[u8] = b"Goodbye, FUSE!\n";
const FILE_TXT_SIZE: usize = FILE_TXT_CONTENTS.len();

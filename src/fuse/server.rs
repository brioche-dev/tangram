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
pub async fn lookup(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn getattr(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn readlink(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn open(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn read(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn opendir(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn readdir(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn access(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

#[tracing::instrument]
pub async fn statfs(request: request::Request<'_>) -> response::Response {
	response::Response::error(libc::ENOSYS)
}

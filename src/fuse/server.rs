use super::{request, response};

#[tracing::instrument]
pub async fn initialize(request: request::Request<'_>) -> response::Response {
	response::Response::error(0)
}

#[tracing::instrument]
pub async fn destroy(request: request::Request<'_>) {}

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

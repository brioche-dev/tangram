#![allow(dead_code)]
use crate::error::Result;

use tokio::io::AsyncReadExt;
mod abi;
mod argument;
mod request;
mod response;
mod server;

/// Run the FUSE file server, listening on `file`.
#[tracing::instrument]
pub async fn run(mut fuse_device: tokio::fs::File) -> Result<()> {
	let mut buffer = aligned_buffer();
	loop {
		eprintln!("Trying to read next message from /dev/fuse.");
		match fuse_device.read(buffer.as_mut()).await {
			Ok(request_size) => {
				// Attempt to deserialize the request.
				let request = request::Request::deserialize(&buffer[..request_size]);
				if let Err(err) = request {
					tracing::error!(?err, "Failed to deserialize FUSE request.");
					continue;
				}
				let request = request.unwrap();

				// Get a response to the request. Failures need to be encapsulated in the response.
				let unique = request.header.unique;

				// FUSE_DESTROY is special in that it does not have a response.
				if let request::RequestData::Destroy = request.data {
					server::destroy(request).await;
					return Ok(());
				} else {
					let response = handle_request(request).await;
					response.write(unique, &mut fuse_device).await?;
					eprintln!("Handled.");
				}
			},
			// If the error is ENOENT, EINTR, or EAGAIN, retry. If ENODEV then the FUSE has been unmounted. Otherwise, return an error.
			Err(e) => match e.raw_os_error() {
				Some(libc::ENOENT) | Some(libc::EINTR) | Some(libc::EAGAIN) => (),
				Some(libc::ENODEV) => return Ok(()),
				_ => Err(e)?,
			},
		};
	}
}

/// Dispatch to one of the response handlers.
async fn handle_request(request: request::Request<'_>) -> response::Response {
	match request.data {
		request::RequestData::Initialize(arg) => server::initialize(request, arg).await,
		request::RequestData::Lookup(_data) => server::lookup(request).await,
		request::RequestData::GetAttr => server::getattr(request).await,
		request::RequestData::ReadLink => server::readlink(request).await,
		request::RequestData::Open(_data) => server::open(request).await,
		request::RequestData::Read(_data) => server::read(request).await,
		request::RequestData::OpenDir(_data) => server::opendir(request).await,
		request::RequestData::ReadDir(_data) => server::readdir(request).await,
		request::RequestData::Access(_data) => server::access(request).await,
		request::RequestData::StatFs => server::statfs(request).await,
		_ => {
			tracing::error!("Unexpected request.");
			unreachable!();
		}
	}
}

pub(crate) const MAX_WRITE_SIZE: usize = 16 * 1024 * 1024;

// We need to create an aligned buffer to write requests into to avoid UB.
fn aligned_buffer() -> Box<[u8]> {
	// MAX_WRITE_SIZE + 1 page.
	let buffer_size = MAX_WRITE_SIZE + 4096;
	let alignment = std::mem::align_of::<abi::fuse_in_header>();
	let ptr = unsafe {
		std::alloc::alloc_zeroed(
			std::alloc::Layout::from_size_align(buffer_size, alignment).unwrap(),
		)
	};
	let ptr = core::ptr::slice_from_raw_parts_mut(ptr, buffer_size);
	unsafe { Box::from_raw(ptr) }
}

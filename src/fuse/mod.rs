use std::sync::Arc;

use crate::{
	error::{error, Result},
	instance::Instance,
};

use tokio::io::AsyncReadExt;
use zerocopy::AsBytes;

use self::{
	request::{Arg, Request},
	response::Response,
};
mod abi;
mod fs;
mod request;
mod response;

/// Run the FUSE file server, listening on `file`.
pub async fn run(mut fuse_device: tokio::fs::File, tg: Arc<Instance>) -> Result<()> {
	let mut buffer = aligned_buffer();
	let file_system = fs::Server::new(tg);
	let mut initialized = false;
	loop {
		// TODO: use synchronous reads.
		match fuse_device.read(buffer.as_mut()).await {
			Ok(request_size) => {
				// Attempt to deserialize the request.
				let request = Request::deserialize(&buffer[..request_size])
					.ok_or(error!("Failed to deserialize FUSE request."));
				if request.is_err() {
					let message = &buffer[..request_size];
					tracing::error!(?message, "Failed to deserialize FUSE request.");
					continue;
				}
				let request = request.unwrap();

				// Get a response to the request. Failures need to be encapsulated in the response.
				let unique = request.header.unique;

				let outfile: std::fs::File =
					fuse_device.try_clone().await.unwrap().into_std().await;

				match request.arg {
					// Perform the initialization handshake.
					Arg::Initialize(arg) => {
						let response = initialize(arg);
						response.write(unique, outfile).await?;
						initialized = true;
					},

					// Drop any requests that occur before the handshake has completed.
					_ if !initialized => {
						tracing::warn!(
							?request,
							"Ignoring request sent before server was initialized."
						);
						Response::error(libc::EIO).write(unique, outfile).await?;
					},

					// Exit.
					Arg::Destroy => return Ok(()),

					// Service the request.
					_ => {
						let file_system = file_system.clone();
						tokio::spawn(async move {
							let response = file_system.handle_request(request.clone()).await;
							if let Err(e) = response.write(unique, outfile).await {
								tracing::error!(?request, ?response, ?e, "Dropped FUSE request.");
							}
						});
					},
				};
			},
			// If the error is ENOENT, EINTR, or EAGAIN, retry. If ENODEV then the FUSE has been unmounted. Otherwise, return an error.
			Err(e) => match e.raw_os_error() {
				Some(libc::ENOENT | libc::EINTR | libc::EAGAIN) => (),
				Some(libc::ENODEV) => return Ok(()),
				_ => Err(e)?,
			},
		};
	}
}

const MAX_WRITE_SIZE: usize = 4096;

// We need to create an aligned buffer to write requests into to avoid UB.
fn aligned_buffer() -> Box<[u8]> {
	// MAX_WRITE_SIZE + 1 page.
	let buffer_size = 16 * 1024 * 1024 + 4096;
	let alignment = std::mem::align_of::<abi::fuse_in_header>();
	let ptr = unsafe {
		std::alloc::alloc_zeroed(
			std::alloc::Layout::from_size_align(buffer_size, alignment).unwrap(),
		)
	};
	let ptr = core::ptr::slice_from_raw_parts_mut(ptr, buffer_size);
	unsafe { Box::from_raw(ptr) }
}

#[tracing::instrument]
fn initialize(arg: abi::fuse_init_in) -> Response {
	let response = abi::fuse_init_out {
		major: 7,                                // Major version that we support.
		minor: 21,                               // Minor version that we target.
		max_readahead: arg.max_readahead,        // Reuse from the argument.
		max_write: MAX_WRITE_SIZE as u32,        // This is a limit on the size of messages.
		flags: abi::consts::FUSE_DO_READDIRPLUS, // Use readdir+ instead of readdir.
		unused: 0,                               // Padding.
	};

	Response::data(response.as_bytes())
}

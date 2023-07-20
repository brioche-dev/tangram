use super::{Blob, Kind};
use crate::{
	error::{return_error, Result},
	instance::Instance,
	util::unsafe_sync::UnsafeSync,
};
use futures::future::BoxFuture;
use num_traits::ToPrimitive;
use pin_project::pin_project;
use std::{io::Cursor, pin::Pin, task::Poll};
use tokio::io::{AsyncRead, AsyncSeek};

impl Blob {
	#[must_use]
	pub fn reader(&self, tg: &Instance) -> Reader {
		Reader {
			blob: self.clone(),
			tg: tg.clone(),
			position: 0,
			state: State::Empty,
		}
	}
}

/// A blob reader.
#[pin_project]
pub struct Reader {
	/// The blob.
	blob: Blob,

	/// The instance.
	tg: Instance,

	/// The position.
	position: u64,

	/// The state.
	state: State,
}

pub enum State {
	Empty,
	Reading(UnsafeSync<BoxFuture<'static, Result<Cursor<Vec<u8>>>>>),
	Full(Cursor<Vec<u8>>),
}

impl AsyncRead for Reader {
	fn poll_read(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> Poll<std::io::Result<()>> {
		let this = self.project();
		loop {
			match this.state {
				State::Empty => {
					if *this.position == this.blob.size() {
						return Poll::Ready(Ok(()));
					}
					let future = {
						let blob = this.blob.clone();
						let tg = this.tg.clone();
						let position = *this.position;
						async move {
							let mut current_blob = blob.clone();
							let mut current_blob_position = 0;
							let block = 'outer: loop {
								match &current_blob.kind {
									Kind::Branch(sizes) => {
										for (block, size) in sizes {
											if position < current_blob_position + size {
												current_blob = Blob::get(&tg, *block).await?;
												continue 'outer;
											}
											current_blob_position += size;
										}
										return_error!("The position is out of bounds.");
									},
									Kind::Leaf(size) => {
										if position < current_blob_position + size {
											let data = current_blob.block().data(&tg).await?;
											let mut reader = Cursor::new(data);
											reader.set_position(position - current_blob_position);
											break reader;
										}
										return_error!("The position is out of bounds.");
									},
								}
							};
							Ok(block)
						}
					};
					let future = Box::pin(future);
					*this.state = State::Reading(UnsafeSync::new(future));
				},

				State::Reading(future) => match future.as_mut().poll(cx) {
					Poll::Pending => return Poll::Pending,
					Poll::Ready(Err(error)) => {
						return Poll::Ready(Err(std::io::Error::new(
							std::io::ErrorKind::Other,
							error,
						)))
					},
					Poll::Ready(Ok(data)) => {
						*this.state = State::Full(data);
					},
				},

				State::Full(reader) => {
					let data = reader.get_ref();
					let position = reader.position().to_usize().unwrap();
					let n = std::cmp::min(buf.remaining(), data.len() - position);
					buf.put_slice(&data[position..position + n]);
					*this.position += n as u64;
					let position = position + n;
					reader.set_position(position as u64);
					if position == reader.get_ref().len() {
						*this.state = State::Empty;
					} else {
						return Poll::Ready(Ok(()));
					}
				},
			};
		}
	}
}

impl AsyncSeek for Reader {
	fn start_seek(self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
		let this = self.project();
		let position = match position {
			std::io::SeekFrom::Start(position) => position.to_i64().unwrap(),
			std::io::SeekFrom::End(position) => this.blob.size().to_i64().unwrap() + position,
			std::io::SeekFrom::Current(position) => this.position.to_i64().unwrap() + position,
		};
		let position = position.to_u64().ok_or(std::io::Error::new(
			std::io::ErrorKind::InvalidInput,
			"Attempted to seek to a negative or overflowing position.",
		))?;
		if position > this.blob.size() {
			return Err(std::io::Error::new(
				std::io::ErrorKind::InvalidInput,
				"Attempted to seek to a position beyond the end of the blob.",
			));
		}
		*this.state = State::Empty;
		*this.position = position;
		Ok(())
	}

	fn poll_complete(
		self: Pin<&mut Self>,
		_cx: &mut std::task::Context<'_>,
	) -> Poll<std::io::Result<u64>> {
		Poll::Ready(Ok(self.position))
	}
}

use crate::{blob, bytes::Bytes, return_error, Client, Error, Result};
use futures::{future::BoxFuture, stream::StreamExt, TryStreamExt};
use num::ToPrimitive;
use pin_project::pin_project;
use std::{io::Cursor, pin::Pin, task::Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek};

const MAX_BRANCH_CHILDREN: usize = 1024;

const MAX_LEAF_SIZE: usize = 262_144;

crate::id!();

crate::kind!(Blob);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub enum Value {
	Branch(Vec<(Handle, u64)>),
	Leaf(Bytes),
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Branch(Vec<(Id, u64)>),
	#[tangram_serialize(id = 1)]
	Leaf(Bytes),
}

impl Handle {
	pub async fn with_reader(client: &Client, mut reader: impl AsyncRead + Unpin) -> Result<Self> {
		let mut leaves = Vec::new();
		let mut bytes = vec![0u8; MAX_LEAF_SIZE];
		loop {
			// Read up to `MAX_LEAF_BLOCK_DATA_SIZE` bytes from the reader.
			let mut position = 0;
			loop {
				let n = reader.read(&mut bytes[position..]).await?;
				position += n;
				if n == 0 || position == bytes.len() {
					break;
				}
			}
			if position == 0 {
				break;
			}
			let size = position.to_u64().unwrap();

			// Create, store, and add the leaf.
			let bytes = Bytes::with_slice(&bytes[..position]);
			let leaf = blob::Handle::new_leaf(bytes);
			leaf.id(client).await?;
			leaves.push((leaf, size));
		}

		// Create the tree.
		while leaves.len() > MAX_BRANCH_CHILDREN {
			leaves = futures::stream::iter(leaves)
				.chunks(MAX_BRANCH_CHILDREN)
				.flat_map(|chunk| {
					if chunk.len() == MAX_BRANCH_CHILDREN {
						futures::stream::once(async move {
							let blob = Self::new(chunk);
							let size = blob.size(client).await?;
							Ok::<_, Error>((blob, size))
						})
						.boxed()
					} else {
						futures::stream::iter(chunk.into_iter().map(Result::Ok)).boxed()
					}
				})
				.try_collect()
				.await?;
		}
		let blob = Self::new(leaves);

		Ok(blob)
	}

	#[must_use]
	pub fn new(children: Vec<(Self, u64)>) -> Self {
		match children.len() {
			0 => Self::empty(),
			1 => children.into_iter().next().unwrap().0,
			_ => Self::new_branch(children),
		}
	}

	#[must_use]
	pub fn empty() -> Self {
		Self::with_value(Value::Leaf(Bytes::empty()))
	}

	#[must_use]
	pub fn new_branch(children: Vec<(blob::Handle, u64)>) -> Self {
		Self::with_value(Value::Branch(children))
	}

	#[must_use]
	pub fn new_leaf(bytes: Bytes) -> Self {
		Self::with_value(Value::Leaf(bytes))
	}

	pub async fn size(&self, tg: &Client) -> Result<u64> {
		Ok(match self.value(tg).await? {
			Value::Branch(children) => children.iter().map(|(_, size)| size).sum(),
			Value::Leaf(bytes) => bytes.len().to_u64().unwrap(),
		})
	}

	pub async fn reader(&self, tg: &Client) -> Result<Reader> {
		let size = self.size(tg).await?;
		Ok(Reader {
			blob: self.clone(),
			size,
			tg: tg.clone(),
			position: 0,
			state: State::Empty,
		})
	}

	pub async fn bytes(&self, tg: &Client) -> Result<Vec<u8>> {
		let mut reader = self.reader(tg).await?;
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		Ok(bytes)
	}

	pub async fn text(&self, tg: &Client) -> Result<String> {
		let bytes = self.bytes(tg).await?;
		let string = String::from_utf8(bytes).map_err(Error::other)?;
		Ok(string)
	}
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			blob::Data::Branch(data) => {
				let value = data
					.into_iter()
					.map(|(handle, size)| (blob::Handle::with_id(handle), size))
					.collect::<Vec<_>>();
				blob::Value::Branch(value)
			},
			blob::Data::Leaf(data) => blob::Value::Leaf(data),
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		todo!()
		// let data = match value {
		// 	blob::Value::Branch(branch) => {
		// 		let branch = branch
		// 			.into_iter()
		// 			.map(|(handle, size)| todo!())
		// 			.collect::<Vec<_>>();
		// 		blob::Data::Branch(branch)
		// 	},
		// 	blob::Value::Leaf(leaf) => blob::Data::Leaf(leaf.clone()),
		// };
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		match self {
			Value::Branch(children) => children
				.iter()
				.map(|(child, _)| child.clone().into())
				.collect(),
			Value::Leaf(_) => vec![],
		}
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		match self {
			Data::Branch(children) => children.iter().map(|(child, _)| (*child).into()).collect(),
			Data::Leaf(_) => vec![],
		}
	}
}

/// A blob reader.
#[pin_project]
pub struct Reader {
	blob: blob::Handle,
	size: u64,
	tg: Client,
	position: u64,
	state: State,
}

pub enum State {
	Empty,
	Reading(BoxFuture<'static, Result<Cursor<Bytes>>>),
	Full(Cursor<Bytes>),
}

unsafe impl Sync for State {}

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
					if *this.position == *this.size {
						return Poll::Ready(Ok(()));
					}
					let future = {
						let blob = this.blob.clone();
						let tg = this.tg.clone();
						let position = *this.position;
						async move {
							let mut current_blob = blob.clone();
							let mut current_blob_position = 0;
							let bytes = 'outer: loop {
								match &current_blob.value(&tg).await? {
									Value::Branch(children) => {
										for (child, size) in children {
											if position < current_blob_position + size {
												current_blob = child.clone();
												continue 'outer;
											}
											current_blob_position += size;
										}
										return_error!("The position is out of bounds.");
									},
									Value::Leaf(bytes) => {
										if position
											< current_blob_position + bytes.len().to_u64().unwrap()
										{
											let mut reader = Cursor::new(bytes.clone());
											reader.set_position(position - current_blob_position);
											break reader;
										}
										return_error!("The position is out of bounds.");
									},
								}
							};
							Ok(bytes)
						}
					};
					let future = Box::pin(future);
					*this.state = State::Reading(future);
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
					}
					return Poll::Ready(Ok(()));
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
			std::io::SeekFrom::End(position) => this.size.to_i64().unwrap() + position,
			std::io::SeekFrom::Current(position) => this.position.to_i64().unwrap() + position,
		};
		let position = position.to_u64().ok_or(std::io::Error::new(
			std::io::ErrorKind::InvalidInput,
			"Attempted to seek to a negative or overflowing position.",
		))?;
		if position > *this.size {
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

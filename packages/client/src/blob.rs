use crate::{object, return_error, Client, Error, Result};
use bytes::Bytes;
use futures::{
	future::BoxFuture,
	stream::{self, StreamExt},
	TryStreamExt,
};
use num::ToPrimitive;
use pin_project::pin_project;
use std::{io::Cursor, pin::Pin, task::Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek};

const MAX_BRANCH_CHILDREN: usize = 1024;

const MAX_LEAF_SIZE: usize = 262_144;

crate::id!(Blob);
crate::handle!(Blob);

#[derive(Clone, Copy, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Blob(object::Handle);

#[derive(Clone, Debug)]
pub enum Object {
	Branch(Vec<(Blob, u64)>),
	Leaf(Bytes),
}

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Branch(Vec<(Id, u64)>),
	#[tangram_serialize(id = 1)]
	Leaf(Bytes),
}

#[derive(Clone, Copy, Debug)]
pub enum ArchiveFormat {
	Tar,
	Zip,
}

#[derive(Clone, Copy, Debug)]
pub enum CompressionFormat {
	Bz2,
	Gz,
	Xz,
	Zstd,
}

impl Blob {
	pub async fn with_reader(
		client: &dyn Client,
		mut reader: impl AsyncRead + Unpin,
	) -> Result<Self> {
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
			let bytes = Bytes::copy_from_slice(&bytes[..position]);
			let leaf = Self::new_leaf(bytes);
			leaf.0.store(client).await?;
			leaves.push((leaf, size));
		}

		// Create the tree.
		while leaves.len() > MAX_BRANCH_CHILDREN {
			leaves = stream::iter(leaves)
				.chunks(MAX_BRANCH_CHILDREN)
				.flat_map(|chunk| {
					if chunk.len() == MAX_BRANCH_CHILDREN {
						stream::once(async move {
							let blob = Self::new(chunk);
							let size = blob.size(client).await?;
							Ok::<_, Error>((blob, size))
						})
						.boxed()
					} else {
						stream::iter(chunk.into_iter().map(Result::Ok)).boxed()
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
		Self(object::Handle::with_object(object::Object::Blob(
			Object::Leaf(Bytes::new()),
		)))
	}

	#[must_use]
	pub fn new_branch(children: Vec<(Blob, u64)>) -> Self {
		Self(object::Handle::with_object(object::Object::Blob(
			Object::Branch(children),
		)))
	}

	#[must_use]
	pub fn new_leaf(bytes: Bytes) -> Self {
		Self(object::Handle::with_object(object::Object::Blob(
			Object::Leaf(bytes),
		)))
	}

	pub async fn size(&self, client: &dyn Client) -> Result<u64> {
		match self.object(client).await? {
			Object::Branch(children) => Ok(children.iter().map(|(_, size)| size).sum()),
			Object::Leaf(bytes) => Ok(bytes.len().to_u64().unwrap()),
		}
	}

	pub async fn reader(&self, client: &dyn Client) -> Result<Reader> {
		let size = self.size(client).await?;
		Ok(Reader {
			blob: self.clone(),
			size,
			client: client.clone_box(),
			position: 0,
			state: State::Empty,
		})
	}

	pub async fn bytes(&self, client: &dyn Client) -> Result<Vec<u8>> {
		let mut reader = self.reader(client).await?;
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		Ok(bytes)
	}

	pub async fn text(&self, client: &dyn Client) -> Result<String> {
		let bytes = self.bytes(client).await?;
		let string = String::from_utf8(bytes)?;
		Ok(string)
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Branch(branch) => Data::Branch(
				branch
					.iter()
					.map(|(blob, size)| (blob.expect_id(), *size))
					.collect::<Vec<_>>(),
			),
			Self::Leaf(leaf) => Data::Leaf(leaf.clone()),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			Data::Branch(data) => Self::Branch(
				data.into_iter()
					.map(|(handle, size)| (Blob::with_id(handle), size))
					.collect::<Vec<_>>(),
			),
			Data::Leaf(data) => Self::Leaf(data),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		match self {
			Self::Branch(children) => children
				.iter()
				.map(|(child, _)| child.handle().clone())
				.collect(),
			Self::Leaf(_) => vec![],
		}
	}
}

impl Data {
	pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub(crate) fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize this object with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Data::Branch(children) => children.iter().map(|(child, _)| (*child).into()).collect(),
			Data::Leaf(_) => vec![],
		}
	}
}

/// A blob reader.
#[pin_project]
pub struct Reader {
	blob: Blob,
	client: Box<dyn Client>,
	position: u64,
	size: u64,
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
						let client = this.client.clone_box();
						let position = *this.position;
						async move {
							let mut current_blob = blob.clone();
							let mut current_blob_position = 0;
							let bytes = 'outer: loop {
								match &current_blob.object(client.as_ref()).await? {
									Object::Branch(children) => {
										for (child, size) in children {
											if position < current_blob_position + size {
												current_blob = child.clone();
												continue 'outer;
											}
											current_blob_position += size;
										}
										return_error!("The position is out of bounds.");
									},
									Object::Leaf(bytes) => {
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

impl std::fmt::Display for ArchiveFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Tar => {
				write!(f, ".tar")?;
			},
			Self::Zip => {
				write!(f, ".zip")?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for ArchiveFormat {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			".tar" => Ok(Self::Tar),
			".zip" => Ok(Self::Zip),
			_ => return_error!("Invalid format."),
		}
	}
}

impl From<ArchiveFormat> for String {
	fn from(value: ArchiveFormat) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for ArchiveFormat {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl std::fmt::Display for CompressionFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let string = match self {
			Self::Bz2 => ".bz2",
			Self::Gz => ".gz",
			Self::Xz => ".xz",
			Self::Zstd => ".zstd",
		};
		write!(f, "{string}")?;
		Ok(())
	}
}

impl std::str::FromStr for CompressionFormat {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			".bz2" => Ok(Self::Bz2),
			".gz" => Ok(Self::Gz),
			".xz" => Ok(Self::Xz),
			".zstd" => Ok(Self::Zstd),
			_ => return_error!("Invalid compression format."),
		}
	}
}

impl From<CompressionFormat> for String {
	fn from(value: CompressionFormat) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for CompressionFormat {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

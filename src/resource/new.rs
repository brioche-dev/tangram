use super::{unpack, Data, Resource};
use crate::{block::Block, checksum::Checksum, error::Result, instance::Instance, operation};
use url::Url;

impl Resource {
	pub async fn new(
		tg: &Instance,
		url: Url,
		unpack: Option<unpack::Format>,
		checksum: Option<Checksum>,
		unsafe_: bool,
	) -> Result<Self> {
		// Create the operation data.
		let data = operation::Data::Resource(Data {
			url: url.clone(),
			unpack,
			checksum: checksum.clone(),
			unsafe_,
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();

		// Create the block.
		let block = Block::new(tg, vec![], bytes.as_slice()).await?;

		// Create the download.
		let download = Self {
			block,
			url,
			unpack,
			checksum,
			unsafe_,
		};

		Ok(download)
	}
}

use super::{unpack, Data, Resource};
use crate::{checksum::Checksum, error::Result, instance::Instance, operation};
use url::Url;

impl Resource {
	pub fn new(
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

		// Serialize and hash the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = operation::Hash(crate::hash::Hash::new(&bytes));

		// Add the operation.
		let hash = tg.database.add_operation(hash, &bytes)?;

		// Create the download.
		let download = Self {
			hash,
			url,
			unpack,
			checksum,
			unsafe_,
		};

		Ok(download)
	}
}

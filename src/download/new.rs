use super::{Data, Download};
use crate::{checksum::Checksum, error::Result, instance::Instance, operation};
use url::Url;

impl Download {
	pub async fn new(
		tg: &Instance,
		url: Url,
		unpack: bool,
		checksum: Option<Checksum>,
		is_unsafe: bool,
	) -> Result<Self> {
		// Create the operation data.
		let data = operation::Data::Download(Data {
			url: url.clone(),
			unpack,
			checksum: checksum.clone(),
			is_unsafe,
		});

		// Serialize and hash the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = operation::Hash(crate::hash::Hash::new(&bytes));

		// Add the operation.
		let hash = tg.database.add_operation(hash, &bytes).await?;

		// Create the download.
		let download = Self {
			hash,
			url,
			unpack,
			checksum,
			is_unsafe,
		};

		Ok(download)
	}
}

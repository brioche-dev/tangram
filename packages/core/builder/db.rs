use crate::{expression::Expression, hash::Hash};
use heed::Database;

pub type ExpressionsDatabase =
	Database<heed::types::SerdeJson<Hash>, heed::types::SerdeJson<(Expression, Option<Hash>)>>;

pub type EvaluationsDatabase =
	Database<heed::types::SerdeJson<Hash>, heed::types::SerdeJson<Vec<Hash>>>;

impl<'a> heed::BytesEncode<'a> for Hash {
	type EItem = Hash;

	fn bytes_encode(
		item: &'a Self::EItem,
	) -> Result<std::borrow::Cow<'a, [u8]>, Box<dyn std::error::Error>> {
		Ok(std::borrow::Cow::Borrowed(&item.0))
	}
}

impl<'a> heed::BytesDecode<'a> for Hash {
	type DItem = Hash;

	fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, Box<dyn std::error::Error>> {
		let mut hash: [u8; 32] = Default::default();
		hash.copy_from_slice(bytes);
		let hash = Hash(hash);
		Ok(hash)
	}
}

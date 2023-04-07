pub use self::{algorithm::Algorithm, writer::Writer};

pub mod algorithm;
mod serialize;
pub mod writer;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub enum Checksum {
	Blake3([u8; 32]),
	Sha256([u8; 32]),
}

impl Checksum {
	#[must_use]
	pub fn algorithm(&self) -> Algorithm {
		match self {
			Self::Blake3(_) => Algorithm::Blake3,
			Self::Sha256(_) => Algorithm::Sha256,
		}
	}
}

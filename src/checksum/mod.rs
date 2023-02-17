pub use self::{algorithm::Algorithm, writer::Writer};

pub mod algorithm;
mod artifact;
mod serialize;
pub mod writer;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub enum Checksum {
	Sha256([u8; 32]),
	Blake3([u8; 32]),
}

impl Checksum {
	#[must_use]
	pub fn algorithm(&self) -> Algorithm {
		match self {
			Self::Sha256(_) => Algorithm::Sha256,
			Self::Blake3(_) => Algorithm::Blake3,
		}
	}
}

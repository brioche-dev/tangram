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
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
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

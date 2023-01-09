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
#[serde(tag = "algorithm", content = "value")]
pub enum Checksum {
	#[buffalo(id = 0)]
	#[serde(rename = "sha256", with = "hex")]
	Sha256([u8; 32]),
}

impl Checksum {
	#[must_use]
	pub fn algorithm(&self) -> Algorithm {
		match self {
			Self::Sha256(_) => Algorithm::Sha256,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
	Sha256,
}

#[derive(Debug)]
pub enum Checksummer {
	Sha256(sha2::Sha256),
}

impl Checksummer {
	#[must_use]
	pub fn new(kind: Algorithm) -> Self {
		match kind {
			Algorithm::Sha256 => Self::Sha256(sha2::Sha256::default()),
		}
	}

	pub fn update(&mut self, data: impl AsRef<[u8]>) {
		match self {
			Self::Sha256(sha256) => sha2::Digest::update(sha256, data),
		}
	}

	#[must_use]
	pub fn finalize(self) -> Checksum {
		match self {
			Self::Sha256(sha256) => {
				let value = sha2::Digest::finalize(sha256);
				Checksum::Sha256(value.into())
			},
		}
	}
}

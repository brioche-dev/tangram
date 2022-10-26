use num_traits::{FromPrimitive, ToPrimitive};

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
pub struct Digest {
	#[buffalo(id = 0)]
	algorithm: DigestAlgorithm,
	#[buffalo(id = 1)]
	encoding: DigestEncoding,
	#[buffalo(id = 2)]
	value: String,
}

#[derive(
	Debug,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	serde::Deserialize,
	serde::Serialize,
	num_derive::FromPrimitive,
	num_derive::ToPrimitive,
)]
#[serde(rename_all = "camelCase")]
pub enum DigestAlgorithm {
	#[default]
	Sha256 = 0,
}

impl std::fmt::Display for DigestAlgorithm {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Sha256 => write!(f, "SHA256"),
		}
	}
}

impl buffalo::Serialize for DigestAlgorithm {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		let value = self.to_u8().unwrap();
		serializer.serialize_uvarint(value.into())
	}
}

impl buffalo::Deserialize for DigestAlgorithm {
	fn deserialize<R>(deserializer: &mut buffalo::Deserializer<R>) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		let value = deserializer.deserialize_uvarint()?;
		let value = Self::from_u64(value)
			.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid system."))?;
		Ok(value)
	}
}

#[derive(
	Debug,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	serde::Deserialize,
	serde::Serialize,
	num_derive::FromPrimitive,
	num_derive::ToPrimitive,
)]
#[serde(rename_all = "camelCase")]
pub enum DigestEncoding {
	#[default]
	Hexadecimal = 0,
}

impl DigestEncoding {
	fn encode(&self, data: impl AsRef<[u8]>) -> String {
		match self {
			Self::Hexadecimal => hex::encode(data),
		}
	}

	fn decode(&self, string: &str) -> Result<Vec<u8>, DigestDecodeError> {
		match self {
			Self::Hexadecimal => {
				let data = hex::decode(string)?;
				Ok(data)
			},
		}
	}
}

impl buffalo::Serialize for DigestEncoding {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		let value = self.to_u8().unwrap();
		serializer.serialize_uvarint(value.into())
	}
}

impl buffalo::Deserialize for DigestEncoding {
	fn deserialize<R>(deserializer: &mut buffalo::Deserializer<R>) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		let value = deserializer.deserialize_uvarint()?;
		let value = Self::from_u64(value)
			.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Invalid system."))?;
		Ok(value)
	}
}

pub struct DigestHasher {
	hasher: DigestAlgorithmHasher,
	algorithm: DigestAlgorithm,
	encoding: DigestEncoding,
	expected_value: Option<String>,
}

impl DigestHasher {
	pub fn new(expected_digest: Option<Digest>) -> Self {
		let algorithm;
		let encoding;
		let expected_value;

		match expected_digest {
			Some(digest) => {
				algorithm = digest.algorithm;
				encoding = digest.encoding;
				expected_value = Some(digest.value);
			},
			None => {
				algorithm = DigestAlgorithm::default();
				encoding = DigestEncoding::default();
				expected_value = None;
			},
		}

		let hasher = DigestAlgorithmHasher::new(algorithm);

		Self {
			hasher,
			algorithm,
			encoding,
			expected_value,
		}
	}

	pub fn update(&mut self, data: impl AsRef<[u8]>) {
		self.hasher.update(data);
	}

	pub fn finalize_and_validate(self) -> Result<(), DigestError> {
		let actual_bytes = self.hasher.finalize();
		let expected = self
			.expected_value
			.ok_or_else(|| DigestError::MissingValue {
				actual: self.encoding.encode(&actual_bytes),
				algorithm: self.algorithm,
			})?;
		let expected_bytes =
			self.encoding
				.decode(&expected)
				.map_err(|error| DigestError::InvalidValue {
					expected: expected.clone(),
					actual: self.encoding.encode(&actual_bytes),
					algorithm: self.algorithm,
					error,
				})?;

		if expected_bytes == actual_bytes {
			Ok(())
		} else {
			Err(DigestError::Mismatch {
				expected,
				actual: self.encoding.encode(&actual_bytes),
				algorithm: self.algorithm,
			})
		}
	}
}

enum DigestAlgorithmHasher {
	Sha256(sha2::Sha256),
}

impl DigestAlgorithmHasher {
	fn new(algorithm: DigestAlgorithm) -> Self {
		match algorithm {
			DigestAlgorithm::Sha256 => Self::Sha256(sha2::Sha256::default()),
		}
	}

	fn update(&mut self, data: impl AsRef<[u8]>) {
		match self {
			Self::Sha256(sha256) => sha2::Digest::update(sha256, data),
		}
	}

	fn finalize(self) -> Vec<u8> {
		match self {
			Self::Sha256(sha256) => sha2::Digest::finalize(sha256).to_vec(),
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum DigestError {
	#[error("expected {expected}, got {actual} ({algorithm}")]
	Mismatch {
		expected: String,
		actual: String,
		algorithm: DigestAlgorithm,
	},
	#[error("no digest was provided, actual digest was {actual} ({algorithm})")]
	MissingValue {
		actual: String,
		algorithm: DigestAlgorithm,
	},
	#[error("actual digest was {actual} ({algorithm}), expected digest {expected:?} is invalid")]
	InvalidValue {
		expected: String,
		actual: String,
		algorithm: DigestAlgorithm,
		#[source]
		error: DigestDecodeError,
	},
}

#[derive(Debug, thiserror::Error)]
pub enum DigestDecodeError {
	#[error("hexadecimal error: {0}")]
	HexadecimalError(#[from] hex::FromHexError),
}

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
	algorithm: Algorithm,
	#[buffalo(id = 1)]
	encoding: Encoding,
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
pub enum Algorithm {
	#[default]
	Sha256 = 0,
}

impl std::fmt::Display for Algorithm {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Sha256 => write!(f, "SHA256"),
		}
	}
}

impl buffalo::Serialize for Algorithm {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		let value = self.to_u8().unwrap();
		serializer.serialize_uvarint(value.into())
	}
}

impl buffalo::Deserialize for Algorithm {
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
pub enum Encoding {
	#[default]
	Hexadecimal = 0,
}

impl Encoding {
	fn encode(self, data: impl AsRef<[u8]>) -> String {
		match self {
			Self::Hexadecimal => hex::encode(data),
		}
	}

	fn decode(self, string: &str) -> Result<Vec<u8>, DecodeError> {
		match self {
			Self::Hexadecimal => {
				let data = hex::decode(string)?;
				Ok(data)
			},
		}
	}
}

impl buffalo::Serialize for Encoding {
	fn serialize<W>(&self, serializer: &mut buffalo::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		let value = self.to_u8().unwrap();
		serializer.serialize_uvarint(value.into())
	}
}

impl buffalo::Deserialize for Encoding {
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

pub struct Hasher {
	hasher: DigestAlgorithmHasher,
	algorithm: Algorithm,
	encoding: Encoding,
	expected_value: Option<String>,
}

impl Hasher {
	#[must_use]
	pub fn new(expected_digest: Option<Digest>) -> Self {
		let algorithm;
		let encoding;
		let expected_value;

		if let Some(digest) = expected_digest {
			algorithm = digest.algorithm;
			encoding = digest.encoding;
			expected_value = Some(digest.value);
		} else {
			algorithm = Algorithm::default();
			encoding = Encoding::default();
			expected_value = None;
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

	pub fn finalize_and_validate(self) -> Result<(), Error> {
		let actual_bytes = self.hasher.finalize();
		let expected = self.expected_value.ok_or_else(|| Error::MissingValue {
			actual: self.encoding.encode(&actual_bytes),
			algorithm: self.algorithm,
		})?;
		let expected_bytes =
			self.encoding
				.decode(&expected)
				.map_err(|error| Error::InvalidValue {
					expected: expected.clone(),
					actual: self.encoding.encode(&actual_bytes),
					algorithm: self.algorithm,
					error,
				})?;

		if expected_bytes == actual_bytes {
			Ok(())
		} else {
			Err(Error::Mismatch {
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
	fn new(algorithm: Algorithm) -> Self {
		match algorithm {
			Algorithm::Sha256 => Self::Sha256(sha2::Sha256::default()),
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
pub enum Error {
	#[error("expected {expected}, got {actual} ({algorithm}")]
	Mismatch {
		expected: String,
		actual: String,
		algorithm: Algorithm,
	},
	#[error("no digest was provided, actual digest was {actual} ({algorithm})")]
	MissingValue {
		actual: String,
		algorithm: Algorithm,
	},
	#[error("actual digest was {actual} ({algorithm}), expected digest {expected:?} is invalid")]
	InvalidValue {
		expected: String,
		actual: String,
		algorithm: Algorithm,
		#[source]
		error: DecodeError,
	},
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
	#[error("hexadecimal error: {0}")]
	HexadecimalError(#[from] hex::FromHexError),
}

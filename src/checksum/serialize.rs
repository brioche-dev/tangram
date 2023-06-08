use super::{Algorithm, Checksum, Encoding};
use crate::error::{return_error, Error, WrapErr};
use base64::Engine;

impl std::fmt::Display for Checksum {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.algorithm, hex::encode(&self.bytes))?;
		Ok(())
	}
}

impl std::str::FromStr for Checksum {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		// Split on a ":" or "-".
		let mut components = if value.contains(':') {
			value.split(':')
		} else {
			value.split('-')
		};

		// Parse the algorithm.
		let algorithm = components
			.next()
			.unwrap()
			.parse()
			.wrap_err("Invalid algorithm.")?;

		// Get the encoded bytes.
		let encoded_bytes = components
			.next()
			.wrap_err(r#"The string must have a ":" or "-" separator."#)?;

		// Determine the encoding.
		let encoding = match (algorithm, encoded_bytes.len()) {
			(Algorithm::Blake3 | Algorithm::Sha256, 64) | (Algorithm::Sha512, 128) => Encoding::Hex,
			(Algorithm::Blake3 | Algorithm::Sha256, 44) | (Algorithm::Sha512, 88) => {
				Encoding::Base64
			},
			_ => return_error!("Invalid checksum string length."),
		};

		// Decode the bytes.
		let bytes = match encoding {
			Encoding::Base64 => base64::engine::general_purpose::STANDARD
				.decode(encoded_bytes)
				.ok()
				.wrap_err(r#"Invalid base64 string."#)?
				.into_boxed_slice(),
			Encoding::Hex => hex::decode(encoded_bytes)
				.ok()
				.wrap_err(r#"Invalid hex string."#)?
				.into_boxed_slice(),
		};

		// Create the checksum.
		let checksum = Checksum { algorithm, bytes };

		Ok(checksum)
	}
}

impl From<Checksum> for String {
	fn from(value: Checksum) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Checksum {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

use base64::Engine;

use super::{Algorithm, Checksum};
use crate::error::{return_error, Error, WrapErr};

impl std::fmt::Display for Checksum {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Checksum::Sha256(bytes) => {
				write!(f, "sha256:{}", hex::encode(bytes))?;
			},
			Checksum::Sha512(bytes) => {
				write!(f, "sha512:{}", hex::encode(bytes))?;
			},
			Checksum::Blake3(bytes) => {
				write!(f, "blake3:{}", hex::encode(bytes))?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for Checksum {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		// Split on a ":" or "-".
		let mut components = if value.contains(':') {
			value.split(":")
		} else {
			value.split("-")
		};

		// Parse the algorithm.
		let algorithm = components
			.next()
			.wrap_err(r#"The string must have a ":"."#)?
			.parse()
			.wrap_err("Invalid algorithm.")?;

		// Parse the bytes.
		let hash_string = components
			.next()
			.wrap_err(r#"The string must have a ":" or "-"."#)?;

		// Check the length of the string and decide if it's base64 or hex.
		let is_base64_string = match (algorithm, hash_string.len()) {
			(Algorithm::Blake3 | Algorithm::Sha256, 64) => false,
			(Algorithm::Blake3 | Algorithm::Sha256, 44) => true,
			(Algorithm::Sha512, 128) => false,
			(Algorithm::Sha512, 88) => true,
			_ => return_error!("Invalid checksum string length."),
		};

		// Decode the string into bytes.
		let bytes = if is_base64_string {
			base64::engine::general_purpose::STANDARD
				.decode(hash_string)
				.ok()
				.wrap_err(r#"Invalid base64 string."#)?
				.into_boxed_slice()
		} else {
			hex::decode(hash_string)
				.ok()
				.wrap_err(r#"Invalid hex string."#)?
				.into_boxed_slice()
		};

		let checksum = match algorithm {
			Algorithm::Sha256 => {
				Checksum::Sha256(bytes)
			},
			Algorithm::Sha512 => {
				Checksum::Sha512(bytes)
			},
			Algorithm::Blake3 => {
				Checksum::Blake3(bytes)
			},
		};

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

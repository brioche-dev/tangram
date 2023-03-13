use super::{Algorithm, Checksum};
use crate::error::{Context, Error};

impl std::fmt::Display for Checksum {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Checksum::Sha256(bytes) => {
				write!(f, "sha256:{}", hex::encode(bytes))?;
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
		// Split on a ":".
		let mut components = value.split(':');

		// Parse the algorithm.
		let algorithm = components
			.next()
			.context(r#"The string must have a ":"."#)?
			.parse()
			.context("Invalid algorithm.")?;

		// Parse the bytes.
		let bytes = hex::decode(
			components
				.next()
				.context(r#"The string must have a ":"."#)?,
		)
		.ok()
		.context("Invalid bytes.")?
		.try_into()
		.ok()
		.context("Invalid bytes.")?;

		let checksum = match algorithm {
			Algorithm::Sha256 => Checksum::Sha256(bytes),
			Algorithm::Blake3 => Checksum::Blake3(bytes),
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

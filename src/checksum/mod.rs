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
	Blake3(Box<[u8]>),
	Sha256(Box<[u8]>),
	Sha512(Box<[u8]>),
}

impl Checksum {
	#[must_use]
	pub fn algorithm(&self) -> Algorithm {
		match self {
			Self::Blake3(_) => Algorithm::Blake3,
			Self::Sha256(_) => Algorithm::Sha256,
			Self::Sha512(_) => Algorithm::Sha512,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn blake3() {
		let data = "Hello, world!";

		let expected_checksum = Checksum::Blake3([
			237, 229, 192, 177, 15, 46, 196, 151, 156, 105, 181, 47, 97, 228, 47, 245, 180, 19, 81,
			156, 224, 155, 224, 241, 77, 9, 141, 207, 229, 246, 249, 141,
		].into());
		let expected_string =
			"blake3:ede5c0b10f2ec4979c69b52f61e42ff5b413519ce09be0f14d098dcfe5f6f98d";

		let mut writer = Writer::new(Algorithm::Blake3);
		writer.update(&data);
		let checksum = writer.finalize();

		assert_eq!(checksum, expected_checksum);
		assert_eq!(&checksum.to_string(), expected_string);
		assert_eq!(
			checksum,
			expected_string
				.parse()
				.expect("Failed to parse blake3 string.")
		);
	}

	#[test]
	fn blake3_sri() {
		let expected_checksum = Checksum::Blake3([
			237, 229, 192, 177, 15, 46, 196, 151, 156, 105, 181, 47, 97, 228, 47, 245, 180, 19, 81,
			156, 224, 155, 224, 241, 77, 9, 141, 207, 229, 246, 249, 141,
		].into());
		let sri = "blake3-7eXAsQ8uxJecabUvYeQv9bQTUZzgm+DxTQmNz+X2+Y0=";
		let checksum: Checksum = sri.parse().expect("Failed to parse blake3 SRI.");
		assert_eq!(checksum, expected_checksum);
	}

	#[test]
	fn sha256() {
		let data = "Hello, world!";

		let expected_checksum = Checksum::Sha256([
			49, 95, 91, 219, 118, 208, 120, 196, 59, 138, 192, 6, 78, 74, 1, 100, 97, 43, 31, 206,
			119, 200, 105, 52, 91, 252, 148, 199, 88, 148, 237, 211,
		].into());
		let expected_string =
			"sha256:315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3";

		let mut writer = Writer::new(Algorithm::Sha256);
		writer.update(&data);
		let checksum = writer.finalize();

		assert_eq!(checksum, expected_checksum);
		assert_eq!(&checksum.to_string(), expected_string);
		assert_eq!(
			checksum,
			expected_string
				.parse()
				.expect("Failed to parse sha256 string.")
		);
	}

	#[test]
	fn sha256_sri() {
		let expected_checksum = Checksum::Sha256([
			49, 95, 91, 219, 118, 208, 120, 196, 59, 138, 192, 6, 78, 74, 1, 100, 97, 43, 31, 206,
			119, 200, 105, 52, 91, 252, 148, 199, 88, 148, 237, 211,
		].into());
		let sri = "sha256-MV9b23bQeMQ7isAGTkoBZGErH853yGk0W/yUx1iU7dM=";
		let checksum: Checksum = sri.parse().expect("Failed to parse sha256 SRI.");
		assert_eq!(checksum, expected_checksum);
	}

	#[test]
	fn sha512() {
		let data = "Hello, world!";

		let expected_checksum = Checksum::Sha512([
			193, 82, 124, 216, 147, 193, 36, 119, 61, 129, 25, 17, 151, 12, 143, 230, 232, 87, 214,
			223, 93, 201, 34, 107, 216, 161, 96, 97, 76, 12, 217, 99, 164, 221, 234, 43, 148, 187,
			125, 54, 2, 30, 249, 216, 101, 213, 206, 162, 148, 168, 45, 212, 154, 11, 178, 105,
			245, 31, 110, 122, 87, 247, 148, 33,
		].into());
		let expected_string = "sha512:c1527cd893c124773d811911970c8fe6e857d6df5dc9226bd8a160614c0cd963a4ddea2b94bb7d36021ef9d865d5cea294a82dd49a0bb269f51f6e7a57f79421";

		let mut writer = Writer::new(Algorithm::Sha512);
		writer.update(&data);
		let checksum = writer.finalize();

		assert_eq!(checksum, expected_checksum);
		assert_eq!(&checksum.to_string(), expected_string);
		assert_eq!(
			checksum,
			expected_string
				.parse()
				.expect("Failed to parse sha512 string.")
		);
	}

	#[test]
	fn sha512_sri() {
		let expected_checksum = Checksum::Sha512([
			193, 82, 124, 216, 147, 193, 36, 119, 61, 129, 25, 17, 151, 12, 143, 230, 232, 87, 214,
			223, 93, 201, 34, 107, 216, 161, 96, 97, 76, 12, 217, 99, 164, 221, 234, 43, 148, 187,
			125, 54, 2, 30, 249, 216, 101, 213, 206, 162, 148, 168, 45, 212, 154, 11, 178, 105,
			245, 31, 110, 122, 87, 247, 148, 33,
		].into());
		let sri = "sha512-wVJ82JPBJHc9gRkRlwyP5uhX1t9dySJr2KFgYUwM2WOk3eorlLt9NgIe+dhl1c6ilKgt1JoLsmn1H256V/eUIQ==";
		let checksum: Checksum = sri.parse().expect("Failed to parse sha512 SRI.");
		assert_eq!(checksum, expected_checksum);
	}
}

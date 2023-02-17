use byteorder::{LittleEndian, NativeEndian, ReadBytesExt};

#[derive(Default)]
pub struct Hasher(Option<u64>);

impl std::hash::Hasher for Hasher {
	fn finish(&self) -> u64 {
		self.0.unwrap()
	}

	fn write(&mut self, mut bytes: &[u8]) {
		if bytes.len() == 8 {
			assert_eq!(bytes.read_u64::<NativeEndian>().unwrap(), 32);
		} else if bytes.len() == 32 {
			self.0 = Some(bytes.read_u64::<LittleEndian>().unwrap());
		} else {
			panic!("Unexpected value to hash.");
		}
	}
}

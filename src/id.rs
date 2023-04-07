#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Id(u128);

impl Id {
	#[must_use]
	pub fn generate() -> Id {
		Id(rand::random())
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:032x?}", self.0)?;
		Ok(())
	}
}

impl std::str::FromStr for Id {
	type Err = std::num::ParseIntError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let id = u128::from_str_radix(s, 16)?;
		let id = Id(id);
		Ok(id)
	}
}

impl serde::Serialize for Id {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> serde::Deserialize<'de> for Id {
	fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct IdVisitor;
		impl<'de> serde::de::Visitor<'de> for IdVisitor {
			type Value = Id;
			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a string")
			}
			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				value.parse().map_err(|_| E::custom("Invalid ID."))
			}
		}
		deserializer.deserialize_str(IdVisitor)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse() {
		let s = "00000000000000000000000000000000";
		assert_eq!(s.parse::<Id>().unwrap().to_string(), s);

		let s = "0000000000000000000000000000000z";
		s.parse::<Id>().unwrap_err();

		let s = "f51a3a61ee9d4731b1b06c816a8ab856";
		assert_eq!(s.parse::<Id>().unwrap().to_string(), s);

		let s = "abc123";
		s.parse::<Id>().unwrap_err();
	}
}

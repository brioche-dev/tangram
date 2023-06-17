use derive_more::{Deref, Display, From, FromStr, Into};

#[derive(
	Clone,
	Copy,
	Debug,
	Default,
	Deref,
	Display,
	Eq,
	From,
	FromStr,
	Into,
	Ord,
	PartialEq,
	PartialOrd,
	tangram_serialize::Serialize,
	tangram_serialize::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[tangram_serialize(into = "crate::hash::Hash", try_from = "crate::hash::Hash")]
pub struct Hash(pub crate::hash::Hash);

impl std::hash::Hash for Hash {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.hash(state);
	}
}

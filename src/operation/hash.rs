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
	buffalo::Serialize,
	buffalo::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[buffalo(into = "crate::hash::Hash", try_from = "crate::hash::Hash")]
pub struct Hash(pub crate::hash::Hash);

impl std::hash::Hash for Hash {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.0.hash(state);
	}
}

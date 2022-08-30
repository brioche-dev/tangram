use crate::id::Id;
use derive_more::{Deref, Display, FromStr};

#[allow(clippy::module_name_repetitions)]
#[derive(
	Clone,
	Copy,
	Debug,
	Deref,
	Display,
	Eq,
	FromStr,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct ReplId(pub Id);

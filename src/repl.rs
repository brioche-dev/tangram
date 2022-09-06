use crate::id::Id;
use derive_more::{Display, FromStr};

#[allow(clippy::module_name_repetitions)]
#[derive(
	Clone,
	Copy,
	Debug,
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

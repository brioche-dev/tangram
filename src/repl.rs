use crate::id::Id;
use derive_more::{Deref, Display};

#[allow(clippy::module_name_repetitions)]
#[derive(
	Display,
	Deref,
	Clone,
	Copy,
	Debug,
	Hash,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct ReplId(pub Id);

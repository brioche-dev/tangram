use crate::id;
use derive_more::{Display, FromStr};

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
pub struct Id(pub id::Id);

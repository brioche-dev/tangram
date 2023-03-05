pub use self::hash::Hash;
use super::dependency;
use crate::artifact;
use std::collections::BTreeMap;

pub mod add;
pub mod get;
pub mod hash;
pub mod serialize;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Instance {
	#[buffalo(id = 0)]
	pub package_hash: artifact::Hash,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<dependency::Specifier, Hash>,
}

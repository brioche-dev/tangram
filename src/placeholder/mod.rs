pub use self::data::Data;

mod data;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Placeholder {
	pub name: String,
}

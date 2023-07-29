pub use self::data::Data;

mod data;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Placeholder {
	pub name: String,
}

impl std::fmt::Display for Placeholder {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, r#"(tg.placeholder {})"#, self.name)?;
		Ok(())
	}
}

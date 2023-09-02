#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Placeholder {
	#[tangram_serialize(id = 0)]
	pub name: String,
}

crate::value!(Placeholder);

// impl std::fmt::Display for Value {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		write!(f, r#"(tg.placeholder {})"#, self.name)?;
// 		Ok(())
// 	}
// }

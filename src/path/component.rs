#[derive(
	Clone,
	Debug,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub enum Component {
	CurrentDir,
	ParentDir,
	Normal(String),
}

impl Component {
	#[must_use]
	pub fn new(string: String) -> Component {
		match string.as_str() {
			"." => Component::CurrentDir,
			".." => Component::ParentDir,
			_ => Component::Normal(string),
		}
	}

	#[must_use]
	pub fn as_str(&self) -> &str {
		match self {
			Component::CurrentDir => ".",
			Component::ParentDir => "..",
			Component::Normal(component) => component,
		}
	}
}

impl Component {
	#[must_use]
	pub fn as_normal(&self) -> Option<&str> {
		match self {
			Component::Normal(name) => Some(name.as_str()),
			_ => None,
		}
	}
}

impl From<&str> for Component {
	fn from(value: &str) -> Self {
		Component::new(value.to_owned())
	}
}

impl From<String> for Component {
	fn from(value: String) -> Self {
		Component::new(value)
	}
}

impl From<Component> for String {
	fn from(value: Component) -> Self {
		value.to_string()
	}
}

impl std::fmt::Display for Component {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())?;
		Ok(())
	}
}

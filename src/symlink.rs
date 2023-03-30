use crate::template::Template;

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
pub struct Symlink {
	#[buffalo(id = 0)]
	pub target: Template,
}

impl Symlink {
	#[must_use]
	pub fn new(target: Template) -> Self {
		Self { target }
	}
}

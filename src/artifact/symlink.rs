use camino::Utf8PathBuf;

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
	pub target: Utf8PathBuf,
}

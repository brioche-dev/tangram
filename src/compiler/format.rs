use super::Compiler;
use anyhow::Result;

impl Compiler {
	#[allow(clippy::unused_async)]
	pub async fn format(&self, text: String) -> Result<String> {
		Ok(text)
	}
}

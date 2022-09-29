use crate::credentials_path;
use anyhow::Result;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	pub email: String,
	pub token: String,
}

impl Credentials {
	pub async fn read() -> Result<Credentials> {
		let path = credentials_path()?;
		if !path.exists() {
			return Ok(Credentials::default());
		}
		let config = tokio::fs::read(&path).await?;
		let config = serde_json::from_slice(&config)?;
		Ok(config)
	}

	pub async fn write(&self) -> Result<()> {
		let path = credentials_path()?;
		tokio::fs::create_dir_all(path.parent().unwrap()).await?;
		let json = serde_json::to_string(self)?;
		tokio::fs::write(&path, &json).await?;
		Ok(())
	}
}

use crate::server::Server;
use anyhow::{bail, Result};
use futures::TryStreamExt;
use std::{path::Path, sync::Arc};

impl Server {
	pub async fn evaluate_fetch(
		self: &Arc<Self>,
		fetch: crate::expression::Fetch,
	) -> Result<crate::value::Value> {
		let response = self.http_client.get(fetch.url.clone()).send().await?;
		let response = response.error_for_status()?;
		let stream = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));
		let mut reader = tokio_util::io::StreamReader::new(stream);
		let unpack = if fetch.unpack {
			Unpack::for_path(fetch.url.path())
		} else {
			None
		};
		todo!()
		// let artifact = todo!();
		// if let Some(hash) = fetch.hash {
		// 	if hash != *artifact.0 {
		// 		bail!(
		// 			"Hash mismatch in fetch!\nExpected: {}\nReceived: {}\n",
		// 			hash,
		// 			artifact.0,
		// 		);
		// 	}
		// } else {
		// 	bail!("Missing hash!\nReceived: {}\n", artifact.0);
		// }
		// // TODO Handle unpacking.
		// Ok(crate::value::Value::Artifact(artifact))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Unpack {
	#[serde(rename = ".tar.bz2")]
	TarBz2,
	#[serde(rename = ".tar.gz")]
	TarGz,
	#[serde(rename = ".tar.xz")]
	TarXz,
	#[serde(rename = "tar.zstd")]
	TarZstd,
	#[serde(rename = ".tar")]
	Tar,
	#[serde(rename = ".zip")]
	Zip,
}

impl Unpack {
	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	pub fn for_path(path: impl AsRef<Path>) -> Option<Unpack> {
		let path = path.as_ref().to_str().unwrap();
		if path.ends_with(".tar.bz2") {
			Some(Unpack::TarBz2)
		} else if path.ends_with(".tar.gz") {
			Some(Unpack::TarGz)
		} else if path.ends_with(".tar.xz") {
			Some(Unpack::TarXz)
		} else if path.ends_with(".tar.zstd") {
			Some(Unpack::TarZstd)
		} else if path.ends_with(".tar") {
			Some(Unpack::Tar)
		} else if path.ends_with(".zip") {
			Some(Unpack::Zip)
		} else {
			None
		}
	}
}

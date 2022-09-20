use super::Server;
use anyhow::{anyhow, Context, Result};
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateAutoshellRequest {
	pub path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DeleteAutoshellRequest {
	pub path: PathBuf,
}

impl Server {
	pub async fn create_autoshell(self: &Arc<Self>, path: &Path) -> Result<()> {
		self.database_transaction(|txn| Self::create_autoshell_with_transaction(txn, path))
			.await
	}

	pub async fn delete_autoshell(self: &Arc<Self>, path: &Path) -> Result<()> {
		self.database_transaction(|txn| Self::delete_autoshell_with_transaction(txn, path))
			.await
	}

	pub async fn get_autoshells(self: &Arc<Self>) -> Result<Vec<PathBuf>> {
		self.database_transaction(Self::get_autoshells_with_transaction)
			.await
	}

	pub fn create_autoshell_with_transaction(
		txn: &rusqlite::Transaction,
		path: &Path,
	) -> Result<()> {
		let path = path
			.to_str()
			.ok_or_else(|| anyhow!("Expected a valid utf-8 path."))?;
		let sql = r#"
			replace into autoshells (
				path
			) values (
				$1
			)
		"#;
		let params = (path,);
		txn.execute(sql, params)?;
		Ok(())
	}

	pub fn delete_autoshell_with_transaction(
		txn: &rusqlite::Transaction,
		path: &Path,
	) -> Result<()> {
		let path = path
			.to_str()
			.ok_or_else(|| anyhow!("Expected a valid utf-8 path."))?;
		let sql = r#"
			delete from
				autoshells
			where
				path = $1
		"#;
		let params = (path,);
		txn.execute(sql, params)?;
		Ok(())
	}

	pub fn get_autoshells_with_transaction(txn: &rusqlite::Transaction) -> Result<Vec<PathBuf>> {
		let sql = r#"
			select
				path
			from autoshells
		"#;
		let mut statement = txn
			.prepare_cached(sql)
			.context("Failed to prepare the query.")?;
		let paths = statement
			.query(())
			.context("Failed to execute the query.")?
			.and_then(|row| {
				let path = row.get::<_, String>(0)?;
				let path = path.parse()?;
				Ok::<_, anyhow::Error>(path)
			})
			.collect::<Result<Vec<_>>>()?;
		Ok(paths)
	}
}

impl Server {
	// Create an autoshell.
	pub(super) async fn handle_create_autoshell_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read request body.")?;

		// Deserialize the request body.
		let CreateAutoshellRequest { path } =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Create the autoshell.
		self.create_autoshell(&path).await?;

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}

	// Delete an autoshell.
	pub(super) async fn handle_delete_autoshell_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read request body.")?;

		// Deserialize the request body.
		let DeleteAutoshellRequest { path } =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Delete the autoshell.
		self.delete_autoshell(&path).await?;

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}

	// List all autoshells.
	pub(super) async fn handle_get_autoshells_request(
		self: &Arc<Self>,
		_request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Get the autoshells.
		let response = self.get_autoshells().await?;

		// Create the response.
		let body = serde_json::to_vec(&response).context("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

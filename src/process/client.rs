use crate::{
	artifact,
	error::{Error, Result, WrapErr},
	template,
	util::{
		fs,
		http::{full, Incoming, Outgoing},
	},
};

use http_body_util::BodyExt;

// type Request = http::Request<Body>;
// type Response = http::Response<hyper::body::Incoming>;

pub struct Client {
	path: fs::PathBuf,
}

impl Client {
	pub fn new() -> Result<Client> {
		let path = fs::PathBuf::from(std::env::var("TANGRAM_SOCKET").map_err(Error::other)?);
		Ok(Client { path })
	}

	async fn request(&self, request: http::Request<Outgoing>) -> Result<http::Response<Incoming>> {
		let stream = tokio::net::UnixStream::connect(&self.path).await?;

		let (mut sender, connection) = hyper::client::conn::http1::handshake(stream)
			.await
			.map_err(Error::other)?;

		tokio::task::spawn(async move {
			connection.await.ok();
		});

		let response = sender.send_request(request).await.map_err(Error::other)?;

		Ok(response)
	}

	pub async fn checkin(&self, path: &fs::Path) -> Result<artifact::Hash> {
		// Create the request.
		let body = serde_json::to_vec(path)
			.map_err(Error::other)
			.wrap_err("Failed to serialize request body.")?;

		let request: http::Request<Outgoing> = http::Request::builder()
			.uri("/v1/checkin")
			.body(full(body))
			.unwrap();

		// Get the response body.
		let response = self
			.request(request)
			.await
			.map_err(Error::other)
			.wrap_err("Failed to perform the request.")?;

		let body = response
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the response body.")?
			.to_bytes();

		// Deserialize the hash and return.
		let hash: artifact::Hash = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize hash.")?;
		Ok(hash)
	}

	pub async fn checkout(&self, hash: artifact::Hash) -> Result<fs::PathBuf> {
		// Create the request.
		let body = serde_json::to_vec(&hash)
			.map_err(Error::other)
			.wrap_err("Failed to serialize request body.")?;

		let request: http::Request<Outgoing> = http::Request::builder()
			.uri("/v1/checkout")
			.body(full(body))
			.unwrap();

		// Get the response body.
		let response = self
			.request(request)
			.await
			.map_err(Error::other)
			.wrap_err("Failed to perform the request.")?;
		let body = response
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the response body.")?
			.to_bytes();

		// Deserialize the path and return.
		let path: fs::PathBuf = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize hash.")?;
		Ok(path)
	}

	pub async fn unrender(&self, string: String) -> Result<template::Template> {
		// Create the request.
		let body = serde_json::to_vec(&string)
			.map_err(Error::other)
			.wrap_err("Failed to serialize request body.")?;

		let request: http::Request<Outgoing> = http::Request::builder()
			.uri("/v1/unrender")
			.body(full(body))
			.unwrap();

		// Get the response body.
		let response = self
			.request(request)
			.await
			.map_err(Error::other)
			.wrap_err("Failed to perform the request.")?;
		let body = response
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the response body.")?
			.to_bytes();

		// Deserialize the template and return.
		let template: template::Template = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize hash.")?;
		Ok(template)
	}
}

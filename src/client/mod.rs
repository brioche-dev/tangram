use crate::server::Server;
use anyhow::{bail, Result};
use graphql_client::Response as GraphQLResponse;
use hyperlocal::UnixClientExt;
use std::{path::PathBuf, sync::Arc};
use url::Url;

mod checkin;
mod checkin_package;
mod checkout;
mod evaluate;
mod repl;

pub enum Client {
	InProcess {
		server: Arc<Server>,
	},
	Unix {
		path: PathBuf,
		client: hyper::Client<hyperlocal::UnixConnector, hyper::Body>,
	},
	Tcp {
		url: Url,
		client: reqwest::Client,
	},
}

impl Client {
	pub async fn new_in_process(path: PathBuf) -> Result<Client> {
		let server = Arc::new(Server::new(&path).await?);
		tokio::spawn(async move { server.serve_unix().await.unwrap() });
		let client = Client::new_unix(path).await?;
		Ok(client)
	}

	pub async fn new_unix(path: PathBuf) -> Result<Client> {
		let client = hyper::Client::unix();
		let client = Client::Unix { path, client };
		Ok(client)
	}

	pub async fn new_tcp(url: Url) -> Result<Client> {
		let client = reqwest::Client::new();
		let client = Client::Tcp { url, client };
		Ok(client)
	}

	async fn request<T>(&self, variables: T::Variables) -> Result<T::ResponseData>
	where
		T: graphql_client::GraphQLQuery,
	{
		let query = T::build_query(variables);
		let response = match self {
			Client::InProcess { .. } => {
				todo!()
			},
			Client::Unix { path, client } => {
				let path = path.join("socket");
				let query = serde_json::to_string(&query)?;
				let body = hyper::Body::from(query);
				let uri = hyperlocal::Uri::new(path, "/graphql");
				let request = http::Request::builder()
					.method(http::Method::POST)
					.header(http::header::CONTENT_TYPE, "application/json")
					.uri(uri)
					.body(body)
					.unwrap();
				let response = client.request(request).await?;
				if !response.status().is_success() {
					let status = response.status();
					bail!("{}", status);
				}
				let body = hyper::body::to_bytes(response).await?;
				let response: GraphQLResponse<T::ResponseData> = serde_json::from_slice(&body)?;
				response
			},
			Client::Tcp { url, client } => {
				let mut url = url.clone();
				url.set_path("/graphql");
				let response = client.post(url).json(&query).send().await?;
				let response: GraphQLResponse<T::ResponseData> = response.json().await?;
				response
			},
		};
		if let Some(errors) = response.errors {
			bail!("{errors:?}");
		}
		let data = if let Some(data) = response.data {
			data
		} else {
			bail!("No data.");
		};
		Ok(data)
	}
}

use super::{
	error::{bad_request, not_found},
	full, BodyStream, Incoming, Outgoing, Server,
};
use crate::{
	block::{self, Block},
	client::block::TryPutBlockOutcome,
	error::{return_error, Error, Result, WrapErr},
};
use futures::TryStreamExt;
use tokio::io::AsyncReadExt;

impl Server {
	pub async fn handle_get_block_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blocks", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};
		let block = Block::with_id(id);

		// Get the block's bytes.
		let Some(bytes) = block.try_get_bytes(&self.tg).await? else {
			return Ok(not_found());
		};

		// Create the body.
		let body = full(bytes);

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
			.unwrap();

		Ok(response)
	}

	pub async fn handle_put_block_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "blocks", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Create a reader from the body.
		let mut body = tokio_util::io::StreamReader::new(
			BodyStream::new(request.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Read the body.
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes).await?;

		// Get the missing children.
		let missing_children = {
			let mut missing_children = Vec::new();
			for id in block.children().await? {
				let id = id?;
				if !Block::with_id(id)
					.try_get_bytes_local(&self.tg)
					.await?
					.is_some()
				{
					missing_children.push(id);
				}
			}
			missing_children
		};

		// If there are no missing children, then store the block.
		if missing_children.is_empty() {
			let block = Block::with_id_and_bytes(id, bytes.into())
				.wrap_err("Failed to create the block.")?;
			block.store(&self.tg).await?;
		}

		// Determine the outcome.
		let outcome = if missing_children.is_empty() {
			TryPutBlockOutcome::Added
		} else {
			TryPutBlockOutcome::MissingChildren(missing_children)
		};

		// Create the body.
		let body = serde_json::to_vec(&outcome).map_err(Error::other)?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}
}

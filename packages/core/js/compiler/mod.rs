use self::runtime::{CheckRequest, Request, Response, Runtime};
use crate::{builder::Builder, hash::Hash};
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

pub mod load;
pub mod resolve;
pub mod runtime;
pub mod transpile;

#[derive(Clone)]
pub struct Compiler {
	state: Arc<State>,
}

pub struct State {
	builder: Builder,
	sender: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Option<Envelope>>>>,
}

struct Envelope {
	request: Request,
	sender: tokio::sync::oneshot::Sender<Result<Response>>,
}

impl Compiler {
	#[must_use]
	pub fn new(builder: Builder) -> Compiler {
		let state = State {
			builder,
			sender: std::sync::Mutex::new(None),
		};
		Compiler {
			state: Arc::new(state),
		}
	}

	fn runtime_sender(&self) -> tokio::sync::mpsc::UnboundedSender<Option<Envelope>> {
		let mut lock = self.state.sender.lock().unwrap();
		if let Some(sender) = lock.as_ref() {
			sender.clone()
		} else {
			// Create a channel to send requests to the compiler runtime.
			let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Option<Envelope>>();

			// Spawn a thread for the compiler runtime to respond to requests.
			std::thread::spawn({
				let builder = self.state.builder.clone();
				move || {
					let runtime = tokio::runtime::Builder::new_current_thread()
						.enable_all()
						.build()
						.unwrap();
					runtime.block_on(async move {
						let mut runtime = Runtime::new(builder);
						while let Some(envelope) = receiver.recv().await {
							// If the received value is `None`, then the thread should terminate.
							let envelope = if let Some(envelope) = envelope {
								envelope
							} else {
								break;
							};

							// Handle the request.
							let response = runtime.handle(envelope.request).await;

							// Send the response.
							envelope.sender.send(response).ok();
						}
					});
				}
			});

			// Save the sender.
			lock.replace(sender.clone());

			sender
		}
	}

	async fn request(&self, request: Request) -> Result<Response> {
		// Create a channel for the compiler runtime to send responses.
		let (sender, receiver) = tokio::sync::oneshot::channel();

		// Send the request.
		let envelope = Envelope { request, sender };
		self.runtime_sender()
			.send(Some(envelope))
			.map_err(|_| anyhow!("Failed to send the request."))?;

		// Receive the response.
		let response = receiver
			.await
			.context("Failed to receive a response for the request.")?
			.context("The handler errored.")?;

		Ok(response)
	}

	pub async fn check(&self, package_hash: Hash) -> Result<Vec<String>> {
		// Create the request.
		let request = Request::Check(CheckRequest { package_hash });

		// Send the request and receive the response.
		let response = self.request(request).await?;
		let response = match response {
			Response::Check(response) => response,
		};

		// Get the result from the response.
		let diagnostics = response.diagnostics;

		Ok(diagnostics)
	}
}

impl Drop for Compiler {
	fn drop(&mut self) {
		if let Some(sender) = self.state.sender.lock().unwrap().take() {
			sender.send(None).ok();
		}
	}
}

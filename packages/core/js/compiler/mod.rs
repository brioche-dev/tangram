use crate::{builder::Builder, hash::Hash};
use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use std::sync::Arc;

pub mod load;
pub mod resolve;
pub mod runtime;
pub mod transpile;

use runtime::{CheckRequest, Envelope, Request, Response};

#[derive(Clone)]
pub struct Compiler {
	state: Arc<State>,
}

pub struct State {
	builder: Builder,
	sender: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Option<Envelope>>>>,
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
						let mut runtime = runtime::Runtime::new(builder);
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

	/// Send an `Request` into the runtime for evaluation.
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

	/// Get all diagnostics for a checked-in package.
	pub async fn check(&self, package_hash: Hash) -> Result<Vec<Diagnostic>> {
		// Get the entrypoint file to this module.
		let entrypoint_file: Utf8PathBuf = self
			.state
			.builder
			.lock_shared()
			.await?
			.resolve_package_entrypoint_file(package_hash)
			.context("Could not resolve package entry point for type-checking")?
			.context("Package has no entrypoint file (e.g. 'tangram.ts').")?;

		// Create the request.
		let files_to_check = vec![
			format!("/__tangram__/module/{package_hash}/{entrypoint_file}"),
			// Also typecheck the target-proxy, because it may contain useful type assertions about
			// this package's targets.
			format!("/__tangram__/target-proxy/{package_hash}/proxy.d.ts"),
		];
		let request = Request::Check(CheckRequest {
			file_names: files_to_check,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// TODO: Remove the #[allow] when there's more than one type of Response.
		#[allow(unreachable_patterns)]
		let response = match response {
			Response::Check(response) => response,
			_ => bail!("Unexpected response type."),
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum Diagnostic {
	File(FileDiagnostic),
	Other(OtherDiagnostic),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileDiagnostic {
	pub file_name: String,
	pub line: u32,
	pub col: u32,
	pub message: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OtherDiagnostic {
	pub message: String,
}

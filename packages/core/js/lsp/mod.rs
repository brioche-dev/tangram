use crate::js;
use anyhow::{bail, Context, Result};
use futures::{future, FutureExt};
use lsp::{notification::Notification, request::Request};
use lsp_types as lsp;
use std::{collections::HashMap, future::Future};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt};

mod completion;
mod definition;
mod diagnostics;
mod files;
mod hover;
mod initialize;
mod jsonrpc;
mod references;
mod types;
mod virtual_text_document;

#[derive(Clone)]
pub struct LanguageServer {
	compiler: js::Compiler,
	outgoing_message_sender: Option<tokio::sync::mpsc::UnboundedSender<jsonrpc::Message>>,
}

impl LanguageServer {
	#[must_use]
	pub fn new(compiler: js::Compiler) -> LanguageServer {
		LanguageServer {
			compiler,
			outgoing_message_sender: None,
		}
	}

	#[allow(clippy::too_many_lines)]
	pub async fn serve(mut self) -> Result<()> {
		let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
		let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());

		// Create a channel to send outgoing messages.
		let (outgoing_message_sender, mut outgoing_message_receiver) =
			tokio::sync::mpsc::unbounded_channel::<jsonrpc::Message>();
		self.outgoing_message_sender = Some(outgoing_message_sender);

		// Create a task to send outgoing messages.
		let outgoing_message_task = tokio::spawn(async move {
			while let Some(outgoing_message) = outgoing_message_receiver.recv().await {
				let body = serde_json::to_string(&outgoing_message)?;
				let head = format!("Content-Length: {}\r\n\r\n", body.len());
				stdout.write_all(head.as_bytes()).await?;
				stdout.write_all(body.as_bytes()).await?;
				stdout.flush().await?;
			}
			Ok::<_, anyhow::Error>(())
		});

		// Read incoming messages.
		loop {
			// Read a message.
			let message = Self::read_incoming_message(&mut stdin).await?;

			// If the message is the exit notification then break.
			if matches!(message,
				jsonrpc::Message::Notification(jsonrpc::Notification {
					ref method,
					..
				}) if method == lsp::notification::Exit::METHOD
			) {
				break;
			};

			// Handle the message.
			self.handle_message(message).await;

			// TODO
			// // Spawn a task to handle the message.
			// tokio::spawn({
			// 	let server = self.clone();
			// 	async move {
			// 		server.handle_message(message).await;
			// 	}
			// });
		}

		// Wait for the outgoing message task to complete.
		outgoing_message_task.await.unwrap()?;

		Ok(())
	}

	async fn read_incoming_message<R>(reader: &mut R) -> Result<jsonrpc::Message>
	where
		R: AsyncRead + AsyncBufRead + Unpin,
	{
		// Read the headers.
		let mut headers = HashMap::new();
		loop {
			let mut line = String::new();
			let n = reader
				.read_line(&mut line)
				.await
				.context("Failed to read a line.")?;
			if n == 0 {
				break;
			}
			if !line.ends_with("\r\n") {
				bail!("Unexpected line ending.");
			}
			let line = &line[..line.len() - 2];
			if line.is_empty() {
				break;
			}
			let mut components = line.split(": ");
			let key = components.next().context("Expected a header name.")?;
			let value = components.next().context("Expected a header value.")?;
			headers.insert(key.to_owned(), value.to_owned());
		}

		// Read and deserialize the message.
		let content_length: usize = headers
			.get("Content-Length")
			.context("Expected a Content-Length header.")?
			.parse()
			.context("Failed to parse the Content-Length header value.")?;
		let mut message: Vec<u8> = vec![0; content_length];
		reader.read_exact(&mut message).await?;
		let message =
			serde_json::from_slice(&message).context("Failed to deserialize the message.")?;

		Ok(message)
	}

	#[allow(clippy::too_many_lines)]
	async fn handle_message(&self, message: jsonrpc::Message) {
		#[allow(clippy::match_same_arms)]
		match message {
			// Handle a request.
			jsonrpc::Message::Request(request) => {
				match request.method.as_str() {
					lsp::request::Completion::METHOD => self
						.handle_request::<lsp::request::Completion, _, _>(request, |params| {
							self.completion(params)
						})
						.boxed(),

					lsp::request::GotoDefinition::METHOD => self
						.handle_request::<lsp::request::GotoDefinition, _, _>(request, |params| {
							self.definition(params)
						})
						.boxed(),

					lsp::request::HoverRequest::METHOD => self
						.handle_request::<lsp::request::HoverRequest, _, _>(request, |params| {
							self.hover(params)
						})
						.boxed(),

					lsp::request::Initialize::METHOD => self
						.handle_request::<lsp::request::Initialize, _, _>(request, |params| {
							self.initialize(params)
						})
						.boxed(),

					lsp::request::References::METHOD => self
						.handle_request::<lsp::request::References, _, _>(request, |params| {
							self.references(params)
						})
						.boxed(),

					lsp::request::Shutdown::METHOD => self
						.handle_request::<lsp::request::Shutdown, _, _>(request, |params| {
							self.shutdown(params)
						})
						.boxed(),

					self::virtual_text_document::VirtualTextDocument::METHOD => self
						.handle_request::<self::virtual_text_document::VirtualTextDocument, _, _>(
							request,
							|params| self.virtual_text_document(params),
						)
						.boxed(),

					// If the request method does not have a handler, send a method not found response.
					_ => {
						self.send_message(jsonrpc::Message::Response(jsonrpc::Response {
							jsonrpc: jsonrpc::VERSION.to_owned(),
							id: request.id,
							result: None,
							error: Some(jsonrpc::ResponseError {
								code: jsonrpc::ResponseErrorCode::MethodNotFound,
								message: "Method not found.".to_owned(),
							}),
						}));
						future::ready(()).boxed()
					},
				}
				.await;
			},

			// Handle a response.
			jsonrpc::Message::Response(_) => {},

			// Handle a notification.
			jsonrpc::Message::Notification(notification) => {
				match notification.method.as_str() {
					lsp::notification::DidOpenTextDocument::METHOD => self
						.handle_notification::<lsp::notification::DidOpenTextDocument, _, _>(
							notification,
							|params| self.did_open(params),
						)
						.boxed(),

					lsp::notification::DidChangeTextDocument::METHOD => self
						.handle_notification::<lsp::notification::DidChangeTextDocument, _, _>(
							notification,
							|params| self.did_change(params),
						)
						.boxed(),

					lsp::notification::DidCloseTextDocument::METHOD => self
						.handle_notification::<lsp::notification::DidCloseTextDocument, _, _>(
							notification,
							|params| self.did_close(params),
						)
						.boxed(),

					// If the notification method does not have a handler, do nothing.
					_ => future::ready(()).boxed(),
				}
				.await;
			},
		}
	}

	async fn handle_request<T, F, Fut>(&self, request: jsonrpc::Request, handler: F)
	where
		T: lsp::request::Request,
		F: Fn(T::Params) -> Fut,
		Fut: Future<Output = Result<T::Result>>,
	{
		// Deserialize the params.
		let params = if let Ok(params) =
			serde_json::from_value(request.params.unwrap_or(serde_json::Value::Null))
		{
			params
		} else {
			self.send_message(jsonrpc::Message::Response(jsonrpc::Response {
				jsonrpc: jsonrpc::VERSION.to_owned(),
				id: request.id,
				result: None,
				error: Some(jsonrpc::ResponseError {
					code: jsonrpc::ResponseErrorCode::InvalidParams,
					message: "Invalid params.".to_owned(),
				}),
			}));
			return;
		};

		// Call the handler.
		let result = handler(params).await;

		// Get the result and error.
		let (result, error) = match result {
			Ok(result) => {
				let result = serde_json::to_value(result).unwrap();
				(Some(result), None)
			},
			Err(error) => {
				let message = error.to_string();
				let error = jsonrpc::ResponseError {
					code: jsonrpc::ResponseErrorCode::InternalError,
					message,
				};
				(None, Some(error))
			},
		};

		// Send the response.
		self.send_message(jsonrpc::Message::Response(jsonrpc::Response {
			jsonrpc: jsonrpc::VERSION.to_owned(),
			id: request.id,
			result,
			error,
		}));
	}

	async fn handle_notification<T, F, Fut>(&self, request: jsonrpc::Notification, handler: F)
	where
		T: lsp::notification::Notification,
		F: Fn(T::Params) -> Fut,
		Fut: Future<Output = Result<()>>,
	{
		let params = serde_json::from_value(request.params.unwrap_or(serde_json::Value::Null))
			.context("Failed to deserialize the request params.")
			.unwrap();
		let result = handler(params).await;
		if let Err(error) = result {
			eprintln!("{error:?}");
		}
	}

	pub fn send_message(&self, message: jsonrpc::Message) {
		if let Some(outgoing_message_sender) = self.outgoing_message_sender.as_ref() {
			outgoing_message_sender.send(message).ok();
		}
	}

	pub fn send_response<T>(
		&self,
		id: jsonrpc::Id,
		result: Option<T::Result>,
		error: Option<jsonrpc::ResponseError>,
	) where
		T: lsp::request::Request,
	{
		let result = result.map(|result| serde_json::to_value(result).unwrap());
		self.send_message(jsonrpc::Message::Response(jsonrpc::Response {
			jsonrpc: jsonrpc::VERSION.to_owned(),
			id,
			result,
			error,
		}));
	}

	pub fn send_notification<T>(&self, params: T::Params)
	where
		T: lsp::notification::Notification,
	{
		let params = serde_json::to_value(params).unwrap();
		self.send_message(jsonrpc::Message::Notification(jsonrpc::Notification {
			jsonrpc: jsonrpc::VERSION.to_owned(),
			method: T::METHOD.to_owned(),
			params: Some(params),
		}));
	}
}

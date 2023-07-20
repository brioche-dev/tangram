use crate::{
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	language::Diagnostic,
};
use futures::{future, FutureExt};
use lsp::{notification::Notification, request::Request};
use lsp_types as lsp;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

mod completion;
mod definition;
mod diagnostics;
mod document;
mod format;
mod hover;
mod initialize;
mod jsonrpc;
mod module;
mod references;
mod rename;
mod symbols;
mod types;
mod virtual_text_document;

type _Receiver = tokio::sync::mpsc::UnboundedReceiver<jsonrpc::Message>;
type Sender = tokio::sync::mpsc::UnboundedSender<jsonrpc::Message>;

#[derive(Clone)]
pub struct Server {
	/// The Tangram instance.
	tg: Instance,

	// The published diagnostics.
	diagnostics: Arc<tokio::sync::RwLock<Vec<Diagnostic>>>,
}

impl Server {
	#[must_use]
	pub fn new(tg: Instance) -> Self {
		let diagnostics = Arc::new(tokio::sync::RwLock::new(Vec::new()));
		Self { tg, diagnostics }
	}

	pub async fn serve(self) -> Result<()> {
		let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
		let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());

		// Create a channel to send outgoing messages.
		let (outgoing_message_sender, mut outgoing_message_receiver) =
			tokio::sync::mpsc::unbounded_channel::<jsonrpc::Message>();

		// Create a task to send outgoing messages.
		let outgoing_message_task = tokio::spawn(async move {
			while let Some(outgoing_message) = outgoing_message_receiver.recv().await {
				let body = serde_json::to_string(&outgoing_message).map_err(Error::other)?;
				let head = format!("Content-Length: {}\r\n\r\n", body.len());
				stdout.write_all(head.as_bytes()).await?;
				stdout.write_all(body.as_bytes()).await?;
				stdout.flush().await?;
			}
			Ok::<_, Error>(())
		});

		// Read incoming messages.
		loop {
			// Read a message.
			let message = read_incoming_message(&mut stdin).await?;

			// If the message is the exit notification, then break.
			if matches!(message,
				jsonrpc::Message::Notification(jsonrpc::Notification {
					ref method,
					..
				}) if method == lsp::notification::Exit::METHOD
			) {
				break;
			};

			// Spawn a task to handle the message.
			tokio::spawn({
				let server = self.clone();
				let sender = outgoing_message_sender.clone();
				async move {
					handle_message(&server, &sender, message).await;
				}
			});
		}

		// Wait for the outgoing message task to complete.
		outgoing_message_task.await.unwrap()?;

		Ok(())
	}
}

async fn read_incoming_message<R>(reader: &mut R) -> Result<jsonrpc::Message>
where
	R: AsyncBufRead + Unpin,
{
	// Read the headers.
	let mut headers = HashMap::new();
	loop {
		let mut line = String::new();
		let n = reader
			.read_line(&mut line)
			.await
			.wrap_err("Failed to read a line.")?;
		if n == 0 {
			break;
		}
		if !line.ends_with("\r\n") {
			return_error!("Unexpected line ending.");
		}
		let line = &line[..line.len() - 2];
		if line.is_empty() {
			break;
		}
		let mut components = line.split(": ");
		let key = components.next().wrap_err("Expected a header name.")?;
		let value = components.next().wrap_err("Expected a header value.")?;
		headers.insert(key.to_owned(), value.to_owned());
	}

	// Read and deserialize the message.
	let content_length: usize = headers
		.get("Content-Length")
		.wrap_err("Expected a Content-Length header.")?
		.parse()
		.map_err(Error::other)
		.wrap_err("Failed to parse the Content-Length header value.")?;
	let mut message: Vec<u8> = vec![0; content_length];
	reader.read_exact(&mut message).await?;
	let message = serde_json::from_slice(&message)
		.map_err(Error::other)
		.wrap_err("Failed to deserialize the message.")?;

	Ok(message)
}

#[allow(clippy::too_many_lines)]
async fn handle_message(server: &Server, sender: &Sender, message: jsonrpc::Message) {
	match message {
		// Handle a request.
		jsonrpc::Message::Request(request) => {
			match request.method.as_str() {
				lsp::request::Completion::METHOD => {
					handle_request::<lsp::request::Completion, _, _>(sender, request, |params| {
						server.completion(params)
					})
					.boxed()
				},

				lsp::request::DocumentSymbolRequest::METHOD => {
					handle_request::<lsp::request::DocumentSymbolRequest, _, _>(
						sender,
						request,
						|params| server.symbols(params),
					)
					.boxed()
				},

				lsp::request::GotoDefinition::METHOD => {
					handle_request::<lsp::request::GotoDefinition, _, _>(
						sender,
						request,
						|params| server.definition(params),
					)
					.boxed()
				},

				lsp::request::Formatting::METHOD => {
					handle_request::<lsp::request::Formatting, _, _>(sender, request, |params| {
						server.format(params)
					})
					.boxed()
				},

				lsp::request::HoverRequest::METHOD => {
					handle_request::<lsp::request::HoverRequest, _, _>(sender, request, |params| {
						server.hover(params)
					})
					.boxed()
				},

				lsp::request::Initialize::METHOD => {
					handle_request::<lsp::request::Initialize, _, _>(
						sender,
						request,
						|params| async move { server.initialize(&params) },
					)
					.boxed()
				},

				lsp::request::References::METHOD => {
					handle_request::<lsp::request::References, _, _>(sender, request, |params| {
						server.references(params)
					})
					.boxed()
				},

				lsp::request::Rename::METHOD => {
					handle_request::<lsp::request::Rename, _, _>(sender, request, |params| {
						server.rename(params)
					})
					.boxed()
				},

				lsp::request::Shutdown::METHOD => handle_request::<lsp::request::Shutdown, _, _>(
					sender,
					request,
					|params| async move { server.shutdown(params) },
				)
				.boxed(),

				self::virtual_text_document::VirtualTextDocument::METHOD => {
					handle_request::<self::virtual_text_document::VirtualTextDocument, _, _>(
						sender,
						request,
						|params| server.virtual_text_document(params),
					)
					.boxed()
				},

				// If the request method does not have a handler, then send a method not found response.
				_ => {
					let error = jsonrpc::ResponseError {
						code: jsonrpc::ResponseErrorCode::MethodNotFound,
						message: "Method not found.".to_owned(),
					};
					send_response::<()>(sender, request.id, None, Some(error));
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
				lsp::notification::DidOpenTextDocument::METHOD => {
					handle_notification::<lsp::notification::DidOpenTextDocument, _, _>(
						sender,
						notification,
						|sender, params| server.did_open(sender, params),
					)
					.boxed()
				},

				lsp::notification::DidChangeTextDocument::METHOD => {
					handle_notification::<lsp::notification::DidChangeTextDocument, _, _>(
						sender,
						notification,
						|sender, params| server.did_change(sender, params),
					)
					.boxed()
				},

				lsp::notification::DidCloseTextDocument::METHOD => {
					handle_notification::<lsp::notification::DidCloseTextDocument, _, _>(
						sender,
						notification,
						|sender, params| server.did_close(sender, params),
					)
					.boxed()
				},

				// If the notification method does not have a handler, then do nothing.
				_ => future::ready(()).boxed(),
			}
			.await;
		},
	}
}

async fn handle_request<T, F, Fut>(sender: &Sender, request: jsonrpc::Request, handler: F)
where
	T: lsp::request::Request,
	F: Fn(T::Params) -> Fut,
	Fut: Future<Output = crate::error::Result<T::Result>>,
{
	// Deserialize the params.
	let Ok(params) = serde_json::from_value(request.params.unwrap_or(serde_json::Value::Null)) else {
		let error = jsonrpc::ResponseError {
			code: jsonrpc::ResponseErrorCode::InvalidParams,
			message: "Invalid params.".to_owned(),
		};
		send_response::<()>(sender, request.id, None, Some(error));
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
	send_response(sender, request.id, result, error);
}

async fn handle_notification<T, F, Fut>(sender: &Sender, request: jsonrpc::Notification, handler: F)
where
	T: lsp::notification::Notification,
	F: Fn(Sender, T::Params) -> Fut,
	Fut: Future<Output = crate::error::Result<()>>,
{
	let params = serde_json::from_value(request.params.unwrap_or(serde_json::Value::Null))
		.map_err(Error::other)
		.wrap_err("Failed to deserialize the request params.")
		.unwrap();
	let result = handler(sender.clone(), params).await;
	if let Err(error) = result {
		eprintln!("{error}");
	}
}

pub fn send_response<T>(
	sender: &Sender,
	id: jsonrpc::Id,
	result: Option<T>,
	error: Option<jsonrpc::ResponseError>,
) where
	T: serde::Serialize,
{
	let result = result.map(|result| serde_json::to_value(result).unwrap());
	let message = jsonrpc::Message::Response(jsonrpc::Response {
		jsonrpc: jsonrpc::VERSION.to_owned(),
		id,
		result,
		error,
	});
	sender.send(message).ok();
}

pub fn send_notification<T>(sender: &Sender, params: T::Params)
where
	T: lsp::notification::Notification,
{
	let params = serde_json::to_value(params).unwrap();
	let message = jsonrpc::Message::Notification(jsonrpc::Notification {
		jsonrpc: jsonrpc::VERSION.to_owned(),
		method: T::METHOD.to_owned(),
		params: Some(params),
	});
	sender.send(message).ok();
}

use crate::{
	expression::{self, Expression},
	hash::Hash,
	server::Server,
	value::Value,
};
use anyhow::Result;
use itertools::Itertools;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use url::Url;

mod cdp;

pub struct Runtime {
	sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

enum Message {
	Repl(ReplMessage),
}

struct ReplMessage {
	code: String,
	sender: tokio::sync::oneshot::Sender<Option<String>>,
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/snapshot"));

/// This is the deno op state for the tangram deno extension.
#[derive(Clone)]
struct TangramOpState {
	main_runtime_handle: tokio::runtime::Handle,
	server: Arc<Server>,
}

impl Runtime {
	pub fn new(server: &Arc<Server>) -> Runtime {
		let server = Arc::clone(&server);

		// Create the channel to send messages to the task.
		let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

		// Create the js runtime task.
		server.local_pool_handle.spawn_pinned({
			let server = Arc::clone(&server);
			move || js_runtime_task(server, receiver)
		});

		Runtime { sender }
	}

	pub async fn repl(&self, code: String) -> Option<String> {
		let (sender, receiver) = tokio::sync::oneshot::channel();
		match self.sender.send(Message::Repl(ReplMessage {
			code: code.clone(),
			sender,
		})) {
			Ok(_) => {},
			Err(_) => return Some("Error: Failed to send the message to the js task.".to_owned()),
		};
		let output = match receiver.await {
			Ok(output) => output,
			Err(_) => return Some("Failed to receive a response from the js task.".to_owned()),
		};
		output
	}
}

async fn js_runtime_task(
	server: Arc<Server>,
	mut receiver: tokio::sync::mpsc::UnboundedReceiver<Message>,
) {
	// Build the tangram extension.
	let tangram_extension = deno_core::Extension::builder()
		.ops(vec![
			op_tangram_console_log::decl(),
			op_tangram_evaluate::decl(),
			op_tangram_fetch::decl(),
			op_tangram_template::decl(),
		])
		.state({
			let server = Arc::clone(&server);
			move |state| {
				state.put(TangramOpState {
					main_runtime_handle: tokio::runtime::Handle::current().clone(),
					server: Arc::clone(&server),
				});
				Ok(())
			}
		})
		.build();

	// Create the js runtime.
	let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
		// module_loader: Some(Rc::new(ModuleLoader::new(Arc::clone(server)))),
		extensions: vec![tangram_extension],
		startup_snapshot: Some(deno_core::Snapshot::Static(SNAPSHOT)),
		..Default::default()
	});

	// Create the v8 inspector session.
	let mut inspector_session = js_runtime.inspector().create_local_session();

	// Enable the inspector runtime.
	futures::try_join!(
		inspector_session.post_message::<()>("Runtime.enable", None),
		js_runtime.run_event_loop(false),
	)
	.unwrap();

	// Retrieve the inspector session context id.
	let mut context_id: u64 = 0;
	for notification in inspector_session.notifications() {
		let method = notification.get("method").unwrap().as_str().unwrap();
		let params = notification.get("params").unwrap();
		if method == "Runtime.executionContextCreated" {
			context_id = params
				.get("context")
				.unwrap()
				.get("id")
				.unwrap()
				.as_u64()
				.unwrap();
		}
	}

	// Respond to each message on the receiver.
	while let Some(message) = receiver.recv().await {
		match message {
			Message::Repl(message) => {
				handle_repl_message(&mut js_runtime, &mut inspector_session, context_id, message)
					.await;
			},
		};
	}
}
#[allow(clippy::too_many_lines)]
async fn handle_repl_message(
	js_runtime: &mut deno_core::JsRuntime,
	inspector_session: &mut deno_core::LocalInspectorSession,
	context_id: u64,
	message: ReplMessage,
) {
	let code = message.code;
	// Wrap the code in parens to make it a ExpressionStatement instead of a BlockStatement to match the behavior of other repls.
	let wrapped_code = if code.trim_start().starts_with('{') && !code.trim_end().ends_with(';') {
		format!("({})", &code)
	} else {
		code
	};
	// Evaluate the code.
	let evaluate_response: cdp::EvaluateResponse = match futures::try_join!(
		inspector_session.post_message(
			"Runtime.evaluate",
			Some(cdp::EvaluateArgs {
				context_id: Some(context_id),
				repl_mode: Some(true),
				expression: wrapped_code,
				object_group: None,
				include_command_line_api: None,
				silent: None,
				return_by_value: None,
				generate_preview: Some(true),
				user_gesture: None,
				await_promise: None,
				throw_on_side_effect: None,
				timeout: None,
				disable_breaks: None,
				allow_unsafe_eval_blocked_by_csp: None,
				unique_context_id: None,
			}),
		),
		js_runtime.run_event_loop(false),
	) {
		Ok((response, _)) => serde_json::from_value(response).unwrap(),
		Err(error) => {
			message.sender.send(Some(error.to_string())).unwrap();
			return;
		},
	};
	if let Some(value) = evaluate_response.result.value {
		let output = serde_json::to_string_pretty(&value).unwrap();
		message.sender.send(Some(output)).unwrap();
		return;
	}
	if let Some(error) = evaluate_response.exception_details {
		message
			.sender
			.send(Some(error.exception.unwrap().description.unwrap()))
			.unwrap();
		return;
	}
	let function = r#"
		 function stringify(value) {
			switch (typeof(value)) {
				case "object":
					if (value instanceof Error) {
						return value.stack;
					}
					if (value instanceof Promise) {
						return "Promise";
					}
					if (value instanceof Array) {
						let object = "[ " + Object.values(value).map(value => stringify(value)).flat().join(", ") + " ]";
						return object;
					} else {
						let object = "{ " + Object.entries(value).map(([key, value]) => `${key}: ${stringify(value)}`).join(", ") + " }";
						return object;
					}
				case "function":
					return `[Function: ${value.name || "(anonymous)"}]`;
				case "undefined":
					return "undefined";
				default:
					return JSON.stringify(value);
			}
		}
	"#;
	let call_function_on_response: cdp::CallFunctionOnResponse = match futures::try_join!(
		inspector_session.post_message(
			"Runtime.callFunctionOn",
			Some(cdp::CallFunctionOnArgs {
				function_declaration: function.to_string(),
				object_id: None,
				arguments: Some(vec![(&evaluate_response.result).into()]),
				silent: None,
				return_by_value: None,
				generate_preview: None,
				user_gesture: None,
				await_promise: None,
				execution_context_id: Some(context_id),
				object_group: None,
				throw_on_side_effect: None
			}),
		),
		js_runtime.run_event_loop(false),
	) {
		Ok((response, _)) => serde_json::from_value(response).unwrap(),
		Err(error) => {
			message.sender.send(Some(error.to_string())).unwrap();
			return;
		},
	};
	if let Some(value) = call_function_on_response.result.value {
		let output = value.as_str().unwrap().to_owned();
		message.sender.send(Some(output)).unwrap();
		return;
	}
	message.sender.send(None).unwrap();
}

#[deno_core::op]
#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_console_log(args: Vec<serde_json::Value>) -> Result<(), deno_core::error::AnyError> {
	let len = args.len();
	for (i, arg) in args.iter().enumerate() {
		print!("{arg}");
		if i != len - 1 {
			print!(" ");
		}
	}
	println!();
	Ok(())
}

#[deno_core::op]
async fn op_tangram_evaluate(
	state: Rc<RefCell<deno_core::OpState>>,
	expression: Expression,
) -> Result<Value, deno_core::error::AnyError> {
	let (main_runtime_handle, server) = {
		let state = state.borrow();
		let tangram_state = state.borrow::<TangramOpState>();
		let main_runtime_handle = tangram_state.main_runtime_handle.clone();
		let server = Arc::clone(&tangram_state.server);
		(main_runtime_handle, server)
	};
	let task = async move {
		let value = server.evaluate(expression).await?;
		Ok::<_, anyhow::Error>(value)
	};
	let value = main_runtime_handle.spawn(task).await.unwrap()?;
	Ok(value)
}

#[derive(serde::Deserialize)]
struct FetchArgs {
	url: Url,
	hash: Option<Hash>,
	unpack: bool,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_fetch(args: FetchArgs) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Fetch(expression::Fetch {
		url: args.url,
		hash: args.hash,
		unpack: args.unpack,
	}))
}

#[derive(serde::Deserialize)]
struct TemplateArgs {
	strings: Vec<String>,
	placeholders: Vec<Expression>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_template(args: TemplateArgs) -> Result<Expression, deno_core::error::AnyError> {
	let components = args
		.strings
		.into_iter()
		.map(Expression::String)
		.interleave(args.placeholders)
		.collect();
	let template = expression::Template { components };
	Ok(Expression::Template(template))
}

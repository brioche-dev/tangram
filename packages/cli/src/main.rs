use self::{commands::Args, util::dirs::home_directory_path};
use clap::Parser;
use std::{path::PathBuf, sync::Arc};
use tangram_client as tg;
use tangram_error::{error, return_error, Result, WrapErr};
use tangram_http::net::Addr;
use tg::Client;
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod tui;
mod util;

pub const API_URL: &str = "https://api.tangram.dev";

struct Cli {
	client: tokio::sync::Mutex<Option<Arc<dyn tg::Client>>>,
	path: PathBuf,
	token: std::sync::RwLock<Option<String>>,
	version: String,
}

#[tokio::main]
async fn main() {
	// Run the main function.
	let result = main_inner().await;

	// Handle the result.
	if let Err(error) = result {
		// Print the error trace.
		eprintln!("An error occurred.");
		eprintln!("{}", error.trace());

		// Exit with a non-zero code.
		std::process::exit(1);
	}
}

async fn main_inner() -> Result<()> {
	// Setup tracing.
	setup_tracing();

	// Initialize V8.
	initialize_v8();

	// Parse the arguments.
	let args = Args::parse();

	// Get the path.
	let path = home_directory_path()
		.wrap_err("Failed to find the user home directory.")?
		.join(".tangram");

	// Create the container for the client.
	let client = tokio::sync::Mutex::new(None);

	// Get the token.
	let credentials = Cli::read_credentials().await?;
	let token = credentials.map(|credentials| credentials.token);
	let token = std::sync::RwLock::new(token);

	// Get the version.
	let version = if cfg!(debug_assertions) {
		let executable_path =
			std::env::current_exe().wrap_err("Failed to get the current executable path.")?;
		let metadata = tokio::fs::metadata(&executable_path)
			.await
			.wrap_err("Failed to get the executable metadata.")?;
		metadata
			.modified()
			.wrap_err("Failed to get the executable modified time.")?
			.duration_since(std::time::SystemTime::UNIX_EPOCH)
			.unwrap()
			.as_secs()
			.to_string()
	} else {
		env!("CARGO_PKG_VERSION").to_owned()
	};

	// Create the CLI.
	let cli = Cli {
		client,
		path,
		token,
		version,
	};

	// Run the command.
	cli.run(args).await?;

	Ok(())
}

impl Cli {
	async fn client(&self) -> Result<Arc<dyn tg::Client>> {
		// If the client is already initialized, return it.
		if let Some(client) = self.client.lock().await.as_ref().cloned() {
			return Ok(client);
		}

		// Attempt to connect to the server.
		let addr = Addr::Unix(self.path.join("socket"));
		let client = tangram_http::client::Builder::new(addr).build();
		let mut connected = client.connect().await.is_ok();

		// If this is a debug build, then require that the client is connected and has the same version as the server.
		if cfg!(debug_assertions) {
			if !connected {
				return_error!("Failed to connect to the server.");
			}
			if connected && client.status().await?.version != self.version {
				return_error!("The server has different version from the client.");
			}
			// Store the client.
			let client = Arc::new(client);
			*self.client.lock().await = Some(client.clone());
			return Ok(client);
		}

		// If the client is connected, check the version.
		if connected && client.status().await?.version != self.version {
			client.stop().await?;
			tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
			client.disconnect().await?;
			connected = false;
		}

		// If the client is not connected, start the server and attempt to connect.
		if !connected {
			self.start_server().await?;
			for _ in 0..10 {
				tokio::time::sleep(std::time::Duration::from_millis(100)).await;
				if client.connect().await.is_ok() {
					connected = true;
					break;
				}
			}
		};

		// If the client is not connected, then return an error.
		if !connected {
			return_error!("Failed to connect to the server.");
		}

		// Store the client.
		let client = Arc::new(client);
		*self.client.lock().await = Some(client.clone());

		Ok(client)
	}

	/// Start the server.
	async fn start_server(&self) -> Result<()> {
		let executable =
			std::env::current_exe().wrap_err("Failed to get the current executable path.")?;
		tokio::fs::create_dir_all(&self.path)
			.await
			.wrap_err("Failed to create the server path.")?;
		let stdout = tokio::fs::File::create(self.path.join("stdout"))
			.await
			.wrap_err("Failed to create the server stdout file.")?;
		let stderr = tokio::fs::File::create(self.path.join("stderr"))
			.await
			.wrap_err("Failed to create the server stderr file.")?;
		tokio::process::Command::new(executable)
			.args(["server", "run"])
			.current_dir(&self.path)
			.stdin(std::process::Stdio::null())
			.stdout(std::process::Stdio::from(stdout.into_std().await))
			.stderr(std::process::Stdio::from(stderr.into_std().await))
			.spawn()
			.wrap_err("Failed to spawn the server.")?;
		Ok(())
	}
}

fn initialize_v8() {
	// Set the ICU data.
	#[repr(C, align(16))]
	struct IcuData([u8; 10_631_872]);
	static ICU_DATA: IcuData = IcuData(*include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../lsp/src/icudtl.dat"
	)));
	v8::icu::set_common_data_73(&ICU_DATA.0).unwrap();

	// Initialize the platform.
	let platform = v8::new_default_platform(0, true);
	v8::V8::initialize_platform(platform.make_shared());

	// Initialize V8.
	v8::V8::initialize();
}

fn setup_tracing() {
	// Create the env layer.
	let tracing_env_filter = std::env::var("TANGRAM_TRACING").ok();
	let env_layer = tracing_env_filter
		.map(|env_filter| tracing_subscriber::filter::EnvFilter::try_new(env_filter).unwrap());

	// If tracing is enabled, create and initialize the subscriber.
	if let Some(env_layer) = env_layer {
		let format_layer = tracing_subscriber::fmt::layer()
			.compact()
			.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NEW)
			.with_writer(std::io::stderr);
		let subscriber = tracing_subscriber::registry()
			.with(env_layer)
			.with(format_layer);
		subscriber.init();
	}
}

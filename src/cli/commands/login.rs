use crate::credentials::{Credentials};
use anyhow::{bail, Result};
use clap::Parser;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[clap(
		long,
		help = "The URL of the API to login to. Defaults to https://api.tangram.dev.",
		default_value = "https://api.tangram.dev"
	)]
	url: Url,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the API client.
	let client = tangram_api_client::Transport::new(&args.uri)?;

	// Create a login.
	let login = client.create_login().await?;

	// Open the browser to the login URL.
	webbrowser::open(&login.login_uri.to_string())?;
	eprintln!(
		"To login, please open your browser to:\n\n{}\n",
		login.login_uri
	);

	// Poll.
	let start_instant = Instant::now();
	let poll_interval = Duration::from_secs(1);
	let poll_duration = Duration::from_secs(300);
	let token = loop {
		if start_instant.elapsed() >= poll_duration {
			bail!("Login timed out. Please try again.");
		}
		let login = client.get_login(login.id).await?;
		if let Some(token) = login.token {
			break token;
		}
		tokio::time::sleep(poll_interval).await;
	};

	// Retrieve the user.
	let user = client.get_current_user(token).await?;

	// Save the credentials.
	let mut credentials = Credentials::read().await?;
	credentials.entries.push(credentials::Entry {
		uri: args.uri.to_string(),
		email: user.email,
		token: user.token,
	});
	credentials.write().await?;

	eprintln!("You have successfully logged in.");

	Ok(())
}

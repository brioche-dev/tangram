use crate::{credentials::Credentials, Cli};
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::time::{Duration, Instant};

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_login(&self, _args: Args) -> Result<()> {
		// Create a login.
		let login = cli
			.api_client
			.create_login()
			.await
			.context("Failed to create the login.")?;

		// Open the browser to the login URL.
		webbrowser::open(login.login_page_url.as_ref())?;
		eprintln!(
			"To login, please open your browser to:\n\n{}\n",
			login.login_page_url
		);

		// Poll.
		let start_instant = Instant::now();
		let poll_interval = Duration::from_secs(1);
		let poll_duration = Duration::from_secs(300);
		let token = loop {
			if start_instant.elapsed() >= poll_duration {
				bail!("Login timed out. Please try again.");
			}
			let login = cli
				.api_client
				.get_login(login.id)
				.await
				.context("Failed to get the login.")?;
			if let Some(token) = login.token {
				break token;
			}
			tokio::time::sleep(poll_interval).await;
		};

		// Retrieve the user.
		let user = cli
			.api_client
			.get_current_user(token)
			.await
			.context("Failed to get the current user.")?;

		// Write the credentials.
		let credentials = Credentials {
			email: user.email,
			token: user.token,
		};
		Self::write_credentials(&credentials)
			.await
			.context("Failed to write the credentials.")?;

		eprintln!("You have successfully logged in.");

		Ok(())
	}
}

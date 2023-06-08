use crate::{
	credentials::Credentials,
	error::{return_error, WrapErr},
	Cli, Result,
};
use std::time::{Duration, Instant};

/// Log in to Tangram.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_login(&self, _args: Args) -> Result<()> {
		// Create a login.
		let login = self
			.tg
			.api_client()
			.create_login()
			.await
			.wrap_err("Failed to create the login.")?;

		// Open the browser to the login URL.
		webbrowser::open(login.url.as_ref())?;
		eprintln!("To log in, please open your browser to:\n\n{}\n", login.url);

		// Poll.
		let start_instant = Instant::now();
		let poll_interval = Duration::from_secs(1);
		let poll_duration = Duration::from_secs(300);
		let token = loop {
			if start_instant.elapsed() >= poll_duration {
				return_error!("Login timed out. Please try again.");
			}
			let login = self
				.tg
				.api_client()
				.get_login(login.id)
				.await
				.wrap_err("Failed to get the login.")?;
			if let Some(token) = login.token {
				break token;
			}
			tokio::time::sleep(poll_interval).await;
		};

		// Get the user.
		let user = self
			.tg
			.api_client()
			.get_current_user(token)
			.await
			.wrap_err("Failed to get the current user.")?;

		// Write the credentials.
		let credentials = Credentials {
			email: user.email,
			token: user.token,
		};
		Self::write_credentials(&credentials).await?;

		eprintln!("You have successfully logged in.");

		Ok(())
	}
}

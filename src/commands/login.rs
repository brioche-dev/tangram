use crate::{credentials::Credentials, return_error, Cli, Result, WrapErr};
use std::time::{Duration, Instant};

/// Log in to Tangram.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {}

impl Cli {
	pub async fn command_login(&self, _args: Args) -> Result<()> {
		// Create a login.
		let login = self
			.client
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
				.client
				.get_login(login.id)
				.await
				.wrap_err("Failed to get the login.")?;
			if let Some(token) = login.token {
				break token;
			}
			tokio::time::sleep(poll_interval).await;
		};

		// Set the token.
		self.client.set_token(Some(token.clone()));

		// Get the user.
		let user = self.client.get_current_user().await?;

		// Write the credentials.
		let credentials = Credentials {
			email: user.email,
			token,
		};
		Self::write_credentials(&credentials).await?;

		eprintln!("You have successfully logged in.");

		Ok(())
	}
}

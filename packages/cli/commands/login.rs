use crate::{credentials::Credentials, Cli};
use anyhow::{bail, Result};
use clap::Parser;
use std::time::{Duration, Instant};
use tangram_api_client::ApiClient;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	pub(crate) async fn command_login(&self, args: Args) -> Result<()> {
		// // Create the API client.
		// let client = ApiClient::new().await?;

		// // Create a login.
		// let login = client.create_login().await?;

		// // Open the browser to the login URL.
		// webbrowser::open(&login.login_uri.to_string())?;
		// eprintln!(
		// 	"To login, please open your browser to:\n\n{}\n",
		// 	login.login_uri
		// );

		// // Poll.
		// let start_instant = Instant::now();
		// let poll_interval = Duration::from_secs(1);
		// let poll_duration = Duration::from_secs(300);
		// let token = loop {
		// 	if start_instant.elapsed() >= poll_duration {
		// 		bail!("Login timed out. Please try again.");
		// 	}
		// 	let login = client.get_login(login.id).await?;
		// 	if let Some(token) = login.token {
		// 		break token;
		// 	}
		// 	tokio::time::sleep(poll_interval).await;
		// };

		// // Retrieve the user.
		// let user = client.get_current_user(token).await?;

		// // Save the credentials.
		// let credentials = Credentials {
		// 	email: user.email,
		// 	token: user.token,
		// };
		// credentials.write().await?;

		// eprintln!("You have successfully logged in.");

		Ok(())
	}
}

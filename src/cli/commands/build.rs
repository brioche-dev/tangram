use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::expression::Expression;
use tracing::Instrument;

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = ".")]
	package: PathBuf,
	#[clap(long, default_value = "build")]
	name: String,
	#[clap(long, takes_value = false)]
	locked: bool,
	args: Vec<String>,
}

pub async fn run(args: Args) -> Result<()> {
	// // Create the client.
	// let client = crate::client::new()
	// 	.await
	// 	.context("Failed to create the client.")?;

	// // Checkin the package.
	// let package = client
	// 	.checkin_package(&args.package, args.locked)
	// 	.await
	// 	.context("Failed to check in the package.")?;

	// // Process the args
	// let arguments = args
	// 	.args
	// 	.into_iter()
	// 	.map(Expression::String)
	// 	.collect::<Vec<_>>();

	// // Evaluate the target.
	// let expression = tangram::expression::Expression::Target(tangram::expression::Target {
	// 	lockfile: None,
	// 	package,
	// 	name: args.name,
	// 	args: arguments,
	// });
	// let value = client
	// 	.evaluate(expression)
	// 	.await
	// 	.context("Failed to evaluate the target expression.")?;

	// // Print the value.
	// let value = serde_json::to_string_pretty(&value).context("Failed to serialize the value.")?;
	// println!("{value}");

	let server = tangram::server::Server::new("/Users/nitsky/.tangram")
		.await
		.unwrap();
	let artifact = "1e7a6cc0eea5a31ce687645032744db753cfd1a1cb249a0307d3ff1c59611d5f"
		.parse()
		.unwrap();

	tokio::try_join!(
		server
			.create_fragment(artifact)
			.instrument(tracing::info_span!("A")),
		server
			.create_fragment(artifact)
			.instrument(tracing::info_span!("B")),
		server
			.create_fragment(artifact)
			.instrument(tracing::info_span!("C")),
	)
	.unwrap();

	Ok(())
}

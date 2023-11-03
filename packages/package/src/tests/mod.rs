#![cfg(test)]
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use tangram_client as tg;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

mod mock;

static MOCK_CLIENT: OnceCell<mock::Client> = once_cell::sync::OnceCell::new();

async fn client() -> &'static mock::Client {
	if MOCK_CLIENT.get().is_none() {
		// setup_tracing();
		let path = std::env::var("TANGRAM_PATH")
			.map(PathBuf::from)
			.unwrap_or_else(|_| {
				let home = PathBuf::from(std::env::var("HOME").unwrap());
				home.join(".tangram")
			});

		// Attempt to connect to the server.
		let addr = tangram_http::net::Addr::Unix(path.join("socket"));
		let client = tangram_http::client::Builder::new(addr).build();
		client.connect().await.unwrap();

		let mock_client = mock::Client::new(&client);
		setup_tests(&mock_client).await;
		let _ = MOCK_CLIENT.set(mock_client);
	}
	MOCK_CLIENT.get().unwrap()
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

// Create & publish the packages for the test harness.
async fn setup_tests(client: &mock::Client) {
	// simple_diamond
	{
		client
			.create_mock_package(
				"simple_diamond_A",
				"1.0.0",
				&[
					tg::dependency::Registry {
						name: "simple_diamond_B".into(),
						version: Some("^1.0".into()),
					},
					tg::dependency::Registry {
						name: "simple_diamond_C".into(),
						version: Some("^1.0".into()),
					},
				],
			)
			.await;
		client
			.create_mock_package(
				"simple_diamond_B",
				"1.0.0",
				&[tg::dependency::Registry {
					name: "simple_diamond_D".into(),
					version: Some("^1.0".into()),
				}],
			)
			.await;
		client
			.create_mock_package(
				"simple_diamond_C",
				"1.0.0",
				&[tg::dependency::Registry {
					name: "simple_diamond_D".into(),
					version: Some("^1.0".into()),
				}],
			)
			.await;
		client
			.create_mock_package("simple_diamond_D", "1.0.0", &[])
			.await;
	}
	// simple_backtrack
	{
		client
			.create_mock_package(
				"simple_backtrack_A",
				"1.0.0",
				&[
					tg::dependency::Registry {
						name: "simple_backtrack_B".into(),
						version: Some("^1.2.3".into()),
					},
					tg::dependency::Registry {
						name: "simple_backtrack_C".into(),
						version: Some("<1.2.3".into()),
					},
				],
			)
			.await;
		client
			.create_mock_package(
				"simple_backtrack_B",
				"1.2.3",
				&[tg::dependency::Registry {
					name: "simple_backtrack_C".into(),
					version: Some("<1.2.3".into()),
				}],
			)
			.await;
		client
			.create_mock_package("simple_backtrack_C", "1.2.3", &[])
			.await;
		client
			.create_mock_package("simple_backtrack_C", "1.2.2", &[])
			.await;
	}
	// diamond_backtrack
	{
		client
			.create_mock_package(
				"diamond_backtrack_A",
				"1.0.0",
				&[
					tg::dependency::Registry {
						name: "diamond_backtrack_B".into(),
						version: Some("1.0.0".into()),
					},
					tg::dependency::Registry {
						name: "diamond_backtrack_C".into(),
						version: Some("1.0.0".into()),
					},
				],
			)
			.await;
		client
			.create_mock_package(
				"diamond_backtrack_B",
				"1.0.0",
				&[tg::dependency::Registry {
					name: "diamond_backtrack_D".into(),
					version: Some("<1.5.0".into()),
				}],
			)
			.await;
		client
			.create_mock_package(
				"diamond_backtrack_C",
				"1.0.0",
				&[tg::dependency::Registry {
					name: "diamond_backtrack_D".into(),
					version: Some("<1.3.0".into()),
				}],
			)
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.1.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.2.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.3.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.4.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.5.0", &[])
			.await;
	}
	// diamond_incompatible_versions
	{}

	// cycle_exists
	{}

	// complex_diamond
	{}
}

#[tokio::test]
async fn version_solving() {
	setup_tracing();
	let client = client().await;
	let _lock = client
		.try_solve(tg::package::Metadata {
			name: Some("simple_diamond_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		})
		.await
		.expect("Failed to solve simple_diamond case.");

	let _lock = client
		.try_solve(tg::package::Metadata {
			name: Some("simple_backtrack_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		})
		.await
		.expect("Failed to solve simple_backtrack case.");

	let _lock = client
		.try_solve(tg::package::Metadata {
			name: Some("diamond_backtrack_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		})
		.await
		.expect("Failed to solve diamond_backtrack case.");
}

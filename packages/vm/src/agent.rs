pub mod guest;
pub mod host;
pub mod proto;

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use tracing::info;

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn test_guest_host_socket() {
		let (client_sock, server_sock) = tokio::io::duplex(64);

		// Start the server.
		let _server_handle = tokio::spawn(async move {
			let mut server = guest::Server::new(server_sock).await.unwrap();
			server.handle().await.unwrap();
			info!("done");
		});

		// Create the client.
		let mut client = host::Client::connect(client_sock).await.unwrap();

		// Send a heartbeat message
		let heartbeat = proto::Heartbeat { nonce: 4242 };
		let response = client.request(heartbeat.clone()).await.unwrap();
		assert_eq!(response, heartbeat.nonce);

		// Test that we can run some shell and read the environment
		let response = client
			.request(proto::RunCommand {
				uid: None,
				executable: "/bin/sh".into(),
				args: vec!["-c".into(), "echo $TEST".into()],
				env: [("TEST".into(), "Hello, World!".into())]
					.into_iter()
					.collect(),
				stdin: vec![],
			})
			.await
			.expect("failed to send msg")
			.expect("failled to run subcommand in guest");
		assert_eq!(response.exit_code, Some(0));
		assert_eq!(response.exit_signal, None);
		assert_eq!(response.stdout, Vec::from("Hello, World!\n".as_bytes()));
		assert_eq!(response.stderr, vec![0u8; 0]);

		// Test that we can run /bin/cat
		let response = client
			.request(proto::RunCommand {
				uid: None,
				executable: "/bin/cat".into(),
				args: vec![],
				env: HashMap::new(),
				stdin: "Hello, World!\n".as_bytes().into(),
			})
			.await
			.expect("failed to send msg")
			.expect("failled to run subcommand in guest");
		assert_eq!(response.exit_code, Some(0));
		assert_eq!(response.exit_signal, None);
		assert_eq!(response.stdout, Vec::from("Hello, World!\n".as_bytes()));
		assert_eq!(response.stderr, vec![0u8; 0]);
	}
}

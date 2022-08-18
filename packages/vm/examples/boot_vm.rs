//! Create and manage `qemu`-backed virtual machines

use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;

use tracing::{debug, info, info_span, Instrument, Level};
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

use std::collections::HashMap;
use tangram_vm::{agent, machine, template::Template, Writability};
use ubyte::ByteUnit;

#[derive(Parser, Debug)]
struct Args {
	/// Path to the VM template manifest.
	template: Utf8PathBuf,

	/// Share a directory from the host to the guest
	#[clap(long)]
	share: Vec<ShareArg>,

	/// Mirror this user from the host (including its SSH keys) to the guest.
	#[clap(long, short('u'))]
	mirror_user: Vec<String>,

	/// Location for Tangram data image.
	#[clap(long)]
	data_image: Option<Utf8PathBuf>,

	/// Commands to execute in a shell once the VM has booted.
	#[clap(long, short)]
	exec: Vec<String>,

	/// After booting, immediately power off the VM
	#[clap(long, short)]
	power_off: bool,

	/// Show verbose logs from the guest.
	#[clap(short, long)]
	verbose: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
	let args = Args::parse();

	FmtSubscriber::builder()
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.with_timer(tracing_subscriber::fmt::time::Uptime::default())
		.with_max_level(if args.verbose {
			Level::DEBUG
		} else {
			Level::INFO
		})
		.init();

	// Load the VM template
	let template = Template::from_manifest(&args.template)
		.await
		.context("Failed to read VM template manifest")?;
	debug!(?template);

	// Create the machine builder.
	// This would be where we configure host-specific things: users, shares, the location of the
	// Tangram data image. All the options that we'd want to change without changing the VM
	// template.
	let mut builder = machine::Builder::new(template)?;

	// If we have one, create and mount the data image
	if let Some(image_path) = &args.data_image {
		// Create the image if it doesn't exist.
		if tokio::fs::metadata(&image_path).await.is_err() {
			let f = tokio::fs::File::create(&image_path)
				.await
				.context("failed to create data image")?;
			f.set_len(ByteUnit::Gigabyte(100).as_u64())
				.await
				.context("failed to resize data image")?;
			f.sync_all()
				.await
				.context("failed to fsync data image after creation")?;
		}

		// Attach it to the VM
		builder.add_disk(machine::Disk {
			image: image_path.clone(),
			id: "tangram-data".into(),
			access: Writability::ReadWrite,
			mount: Some(machine::DiskMount {
				fs: machine::DiskFs::Btrfs,
				mountpoint: "/opt/tangram".into(),
			}),
		});
	}

	// Mirror users
	for username in &args.mirror_user {
		let user = machine::User::from_host_user(username)
			.await
			.with_context(|| format!("failed to load information about host user: {username}"))?;
		builder.add_user(user);
	}

	// Add shares
	for (i, share) in args.share.into_iter().enumerate() {
		builder.add_share(machine::Share {
			tag: format!("share-{i}"),
			host_path: share.host,
			guest_path: share.guest,
			access: share.access,
		});
	}

	// Forward the Tangram socket
	builder.add_socket(machine::Socket {
		name: "tangram-server".into(),
		guest_path: "/opt/tangram/socket".into(),
	});

	debug!(?builder);

	// Run the VM
	let machine = builder.start().await.context("failed to start machine")?;
	info!("Guest online");

	let mut agent = machine
		.agent()
		.await
		.context("failed to connect to agent after machine has booted")?;

	// Get the network info from the guest
	let net_info = agent
		.request(agent::proto::GetNetworkInfo {})
		.await
		.context("Failed to make guest agent request for network info")?
		.context("Failed to gather network information")?;
	info!(ip=%net_info.local_ip, "Guest local IP");
	for iface in net_info.interfaces {
		info!(name=%iface.name, ip=%iface.ip, "Guest network interface");
	}

	// Execute commands
	for cmd in args.exec {
		agent
			.request(agent::proto::RunCommand {
				executable: "/bin/sh".into(),
				args: vec!["-c".into(), cmd.clone().into()],
				env: HashMap::new(),
				stdin: vec![],
				uid: Some(0),
			})
			.instrument(info_span!("Run command", cmd=%&cmd))
			.await
			.context("Failed to make guest-agent request to run --exec command")?
			.context("--exec command failed")?;
	}

	// Print info about forwarded sockets
	info!(
		path = %machine.path_to_socket("tangram-server").unwrap(),
		"Forwarded Tangram server socket"
	);

	// Power off the VM, if required.
	if args.power_off {
		info!("Shutting down guest");
		machine
			.shutdown()
			.await
			.context("failed to power off VM after booting")?;
	} else {
		// Wait for the machine to shut down.
		info!("Waiting for guest shutdown");
		machine
			.wait()
			.await
			.context("failed to wait for machine shutdown")?;
	}

	info!("Guest powered off");

	Ok(())
}

#[derive(Debug)]
struct ShareArg {
	host: Utf8PathBuf,
	guest: Utf8PathBuf,
	access: Writability,
}

impl std::str::FromStr for ShareArg {
	type Err = anyhow::Error;
	fn from_str(source: &str) -> Result<ShareArg> {
		// Split the source on ':'
		let parts: Vec<&str> = source.split(':').collect();

		Ok(match parts.as_slice() {
			&[host, guest] | &[host, guest, "rw"] => {
				ShareArg {
					host: host.into(),
					guest: guest.into(),
					access: Writability::ReadWrite,
				}
			},
			&[host, guest, "ro"] => {
				ShareArg {
					host: host.into(),
					guest: guest.into(),
					access: Writability::ReadOnly,
				}
			},
			_ => bail!("invalid share specification: expected <host_path>:<guest_path>[:ro]"),
		})
	}
}

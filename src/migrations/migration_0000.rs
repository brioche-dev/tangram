use crate::{error::Result, util::fs};
use tokio::io::AsyncWriteExt;

pub(crate) const ENV_AMD64_LINUX: &[u8] = include_bytes!("../../assets/env_amd64_linux");
pub(crate) const ENV_ARM64_LINUX: &[u8] = include_bytes!("../../assets/env_arm64_linux");
pub(crate) const SH_AMD64_LINUX: &[u8] = include_bytes!("../../assets/sh_amd64_linux");
pub(crate) const SH_ARM64_LINUX: &[u8] = include_bytes!("../../assets/sh_arm64_linux");

pub async fn migrate(path: &fs::Path) -> Result<()> {
	// Create the database file.
	let path = path.to_owned();
	tokio::fs::File::create(&path.join("database.mdb")).await?;

	// Open the environment.
	let database_path = path.join("database.mdb");
	let mut env_builder = lmdb::Environment::new();
	env_builder.set_max_dbs(6);
	env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
	let env = env_builder.open(&database_path)?;

	// Create the artifacts database.
	env.create_db("artifacts".into(), lmdb::DatabaseFlags::empty())?;

	// Create the artifact trackers database.
	env.create_db("artifact_trackers".into(), lmdb::DatabaseFlags::empty())?;

	// Create the package instances database.
	env.create_db("package_instances".into(), lmdb::DatabaseFlags::empty())?;

	// Create the operations database.
	env.create_db("operations".into(), lmdb::DatabaseFlags::empty())?;

	// Create the operation children database.
	let mut flags = lmdb::DatabaseFlags::empty();
	flags.insert(lmdb::DatabaseFlags::DUP_SORT);
	flags.insert(lmdb::DatabaseFlags::DUP_FIXED);
	env.create_db("operation_children".into(), flags)?;

	// Create the operation outputs database.
	env.create_db("operation_outputs".into(), lmdb::DatabaseFlags::empty())?;

	// Create the blobs directory.
	let blobs_path = path.join("blobs");
	tokio::fs::create_dir_all(&blobs_path).await?;

	// Create the artifacts directory.
	let artifacts_path = path.join("artifacts");
	tokio::fs::create_dir_all(&artifacts_path).await?;

	// Create the logs directory.
	let logs_path = path.join("logs");
	tokio::fs::create_dir_all(&logs_path).await?;

	// Create the temps directory.
	let temps_path = path.join("temps");
	tokio::fs::create_dir_all(&temps_path).await?;

	// david edit: fill this with bb_arm64_linux, bb_x86_linux, etc. don't ifdef

	// Create the assets directory.
	let assets_path = path.join("assets");
	tokio::fs::create_dir_all(&assets_path).await?;

	// Add `env` and `sh` to the assets.
	let mut opts = tokio::fs::OpenOptions::new();
	opts.create(true).write(true).mode(0o777);

	opts.open(assets_path.join("env_amd64_linux"))
		.await?
		.write_all(ENV_AMD64_LINUX)
		.await?;
	opts.open(assets_path.join("env_arm64_linux"))
		.await?
		.write_all(ENV_ARM64_LINUX)
		.await?;
	opts.open(assets_path.join("sh_amd64_linux"))
		.await?
		.write_all(SH_AMD64_LINUX)
		.await?;
	opts.open(assets_path.join("sh_arm64_linux"))
		.await?
		.write_all(SH_ARM64_LINUX)
		.await?;

	Ok(())
}

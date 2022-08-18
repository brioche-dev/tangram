use anyhow::{bail, Result};
use duct::cmd;
use std::path::Path;
use tangram_io::fs;

pub fn init_nodaemon(root_path: impl AsRef<Path>) -> Result<()> {
	std::fs::create_dir_all(root_path)?;
	Ok(())
}

pub fn uninit_nodaemon(root_path: impl AsRef<Path>) -> Result<()> {
	std::fs::remove_dir_all(root_path)?;
	Ok(())
}

pub fn init_daemon(user_name: &str, group_name: &str, root_path: &Path) -> Result<()> {
	if user_name != group_name {
		// "--user-group" will not work if this isn't true
		bail!("Cannot initialize daemon: username ({user_name:?}) and group name ({group_name:?}) are different");
	}
	// Create the tangram user and group.
	cmd!("useradd", "--system", "--user-group", user_name).run()?;
	// Create the tangram root and set its permissions.
	fs::blocking::create_dir_all(&root_path)?;
	cmd!("chown", format!("{user_name}:{group_name}"), &root_path).run()?;
	cmd!("chmod", "755", &root_path).run()?;
	Ok(())
}

pub fn uninit_daemon(user_name: &str, _group_name: &str, root_path: &Path) -> Result<()> {
	// Remove the tangram root.
	fs::blocking::remove_dir_all(root_path)?;
	// Remote the tangram user.
	// This will also remove the tangram group, as it's this user's primary group.
	cmd!("userdel", user_name).run()?;
	Ok(())
}

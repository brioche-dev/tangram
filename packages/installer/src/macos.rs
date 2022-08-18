use std::path::Path;

use anyhow::Result;
use duct::cmd;
use tangram_io::fs;

const DAEMON_SERVICE_NAME: &str = "com.tangram.Tangram";
const DAEMON_SERVICE_PLIST_PATH: &str = "/Library/LaunchDaemons/dev.tangram.Tangram.plist";

pub fn init_nodaemon(root_path: impl AsRef<Path>) -> Result<()> {
	std::fs::create_dir_all(&root_path)?;
	Ok(())
}

pub fn uninit_nodaemon(root_path: impl AsRef<Path>) -> Result<()> {
	std::fs::remove_dir_all(&root_path)?;
	Ok(())
}

#[allow(clippy::too_many_lines)]
pub fn init_daemon(user_name: &str, group_name: &str, root_path: &Path) -> Result<()> {
	let root_path = root_path.display().to_string();
	// Create the tangram user and group.
	cmd!(
		"dscl",
		".",
		"-create",
		format!("/Groups/{group_name}"),
		"gid",
		"1234",
	)
	.run()?;
	cmd!(
		"dscl",
		".",
		"-create",
		format!("/Groups/{group_name}"),
		"PrimaryGroupId",
		"1234",
	)
	.run()?;
	cmd!(
		"sudo",
		"sysadminctl",
		"-addUser",
		user_name,
		"-fullName",
		"Tangram Daemon",
		// This home directory will be protected even from the superuser by macOS System Integrity Protection.
		"-home",
		"/opt/tangram",
		// This user will not be able to log in to a shell.
		"-shell",
		"/usr/bin/false",
		"-GID",
		"1234",
	)
	.run()?;
	cmd!(
		"sudo",
		"dscl",
		".",
		"-append",
		format!("/Users/{user_name}"),
		// The '*' means a password is required, but does not set one. This prevents the user from logging in via the GUI.
		"password",
		"*"
	)
	.run()?;
	cmd!(
		"sudo",
		"dscl",
		".",
		"-append",
		format!("/Users/{user_name}"),
		// Prevent the user from appearing in user management or in the GUI login screen.
		"IsHidden",
		"1"
	)
	.run()?;
	// Create the tangram root and set its permissions.
	fs::blocking::create_dir_all(&root_path)?;
	cmd!("chown", format!("{}:{}", user_name, group_name), &root_path).run()?;
	cmd!("chmod", "755", &root_path).run()?;

	// TODO Find the correct path to the tangram executable.
	let cli_path = "/usr/local/bin/tg";

	if !Path::new(cli_path).is_file() {
		tracing::warn!(path=?cli_path, "launchd cannot start the daemon, because the binary does not exist");
	}

	let service_plist = indoc::formatdoc!(
		r#"
			<?xml version="1.0" encoding="UTF-8"?>
			<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
			<plist version="1.0">
				<dict>
					<key>Label</key>
					<string>com.tangram.Tangram</string>
					<key>ProgramArguments</key>
					<array>
						<string>{cli_path}</string>
						<string>daemon</string>
					</array>
					<key>StandardOutPath</key>
					<string>{root_path}/out.txt</string>
					<key>StandardErrorPath</key>
					<string>{root_path}/err.txt</string>
					<key>WorkingDirectory</key>
					<string>{root_path}</string>
					<key>RunAtLoad</key>
					<true/>
					<key>KeepAlive</key>
					<true/>
					<key>ThrottleInterval</key>
					<integer>30</integer>
					<key>UserName</key>
					<string>{user_name}</string>
					<key>GroupName</key>
					<string>{group_name}</string>
					<key>InitGroups</key>
					<true/>
					<key>Umask</key>
					<integer>18</integer>
				</dict>
			</plist>
		"#
	);
	fs::blocking::write(&DAEMON_SERVICE_PLIST_PATH, &service_plist)?;
	cmd!("launchctl", "load", &DAEMON_SERVICE_PLIST_PATH).run()?;
	cmd!("launchctl", "start", DAEMON_SERVICE_NAME).run()?;
	Ok(())
}

pub fn uninit_daemon(user_name: &str, _group_name: &str, root_path: &Path) -> Result<()> {
	std::fs::remove_dir_all(&root_path)?;

	// Unregister the daemon.
	cmd!("launchctl", "stop", DAEMON_SERVICE_NAME).run()?;
	cmd!("launchctl", "unload", DAEMON_SERVICE_PLIST_PATH).run()?;
	cmd!("rm", DAEMON_SERVICE_PLIST_PATH).run()?;
	// Delete the tangram user. This command also removes the tangram group and removes the tangram root because it is the tangram user's home directory, which is otherwise protected by SIP.
	cmd!("sysadminctl", "-deleteUser", user_name).run()?;
	Ok(())
}

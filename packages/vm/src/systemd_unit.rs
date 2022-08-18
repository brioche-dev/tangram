use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, SpaceSeparator, StringWithSeparator};

type SpaceSeparated<T> = StringWithSeparator<SpaceSeparator, T>;

/// A systemd service unit file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceUnit {
	pub unit: Unit,
	pub install: Install,
	pub service: Service,
}

impl ServiceUnit {
	pub fn to_conf(&self) -> Result<String> {
		serde_ini::to_string(self).context("failed to format .conf file")
	}
}

/// The systemd unit `[Unit]` section.
///
/// > The unit file may include a [Unit] section, which carries generic information about the unit that is not dependent on the type of unit
///
/// See: <https://www.freedesktop.org/software/systemd/man/systemd.unit.html#%5BUnit%5D%20Section%20Options>
#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Unit {
	pub description: String,

	/// Units that should be activated alongside this one.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub wants: Vec<String>,

	/// Paths that must be mounted before this unit is activated.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub requires_mounts_for: Vec<String>,

	/// Units that must be activated alongside this one.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub requires: Vec<String>,

	/// Units that we must activate before.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub before: Vec<String>,

	/// Units that we must activate after.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub after: Vec<String>,
}

/// The systemd unit `[Service]` section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Service {
	#[serde(rename = "Type", default, skip_serializing_if = "Option::is_none")]
	pub kind: Option<ServiceType>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub restart: Option<ServiceRestart>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub exec_start: Option<String>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub standard_input: Option<String>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub standard_output: Option<String>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub standard_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceType {
	Simple,
	Exec,
	Forking,
	Oneshot,
	Dbus,
	Notify,
	Idle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceRestart {
	No,
	OnSuccess,
	OnFailure,
	OnAbnormal,
	OnWatchdog,
	OnAbort,
	Always,
}

/// The systemd unit `[Install]` section.
///
/// > Unit files may include an [Install] section, which carries installation information for the unit. This section is not interpreted by systemd(1) during runtime; it is used by the enable and disable commands of the systemctl(1) tool during installation of a unit.
///
/// See: <https://www.freedesktop.org/software/systemd/man/systemd.unit.html#%5BInstall%5D%20Section%20Options>
#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Install {
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub wanted_by: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[serde_as(as = "SpaceSeparated<String>")]
	pub required_by: Vec<String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn create_unit() {
		let service = ServiceUnit {
			unit: Unit {
				description: "Example Service".into(),
				..Default::default()
			},
			service: Service {
				exec_start: Some("/mnt/init_data/guest_agent".into()),
				kind: Some(ServiceType::Exec),
				restart: Some(ServiceRestart::OnFailure),
				standard_input: None,
				standard_output: None,
				standard_error: None,
			},
			install: Install {
				wanted_by: vec![
					"multi-user.target".into(),
					"some-other-example.target".into(),
				],
				required_by: vec!["example.target".into()],
			},
		};

		println!("{}", service.to_conf().unwrap());
	}
}

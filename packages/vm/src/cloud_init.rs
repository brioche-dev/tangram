//! Build a `NoCloud` cloud-init image to configure the guest on first boot.
//!
//! See:
//! - <https://cloudinit.readthedocs.io/en/latest/topics/datasources/nocloud.html>
//! - <https://serverfault.com/a/820055>
//! - <https://cloudinit.readthedocs.io/en/latest/topics/examples.html>

use anyhow::{Context, Result};
use fatfs;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
pub struct InitData {
	pub user_data: UserData,
	pub meta_data: MetaData,

	/// Extra files to include in the init disk image
	pub extra_files: BTreeMap<String, Vec<u8>>,
}

impl InitData {
	/// Create an empty set of cloud-init [`InitData`], with the given identifier used as an
	/// instance ID and hostname.
	#[must_use]
	pub fn new(machine_id: &str) -> InitData {
		InitData {
			user_data: UserData::default(),
			meta_data: MetaData {
				instance_id: machine_id.to_owned(),
				local_hostname: machine_id.to_owned(),
			},
			extra_files: std::collections::BTreeMap::new(),
		}
	}

	/// Convert the InitData into a FAT32 disk image.
	#[instrument(name = "InitData::into_disk_image", level = "DEBUG", skip_all)]
	pub fn into_disk_image(self) -> Result<Vec<u8>> {
		debug!(
			user_data=?self.user_data,
			meta_data=?self.meta_data,
			extra_file_names=?self.extra_files.keys().collect::<Vec<_>>()
		);

		// Materialize all the files we'll need first, to compute the final size of the image.
		let mut files: BTreeMap<String, Vec<u8>> = self.extra_files;

		// Create user-data file
		let user_data_string = serde_json::to_string(&self.user_data)
			.context("could not serialize user-data to JSON")?;
		let user_data_buf = format!("#cloud-config\n{user_data_string}").into_bytes();
		files.insert("user-data".into(), user_data_buf);

		// Create meta-data file
		let meta_data_buf =
			serde_json::to_vec(&self.meta_data).context("could not serialize meta-data to JSON")?;
		files.insert("meta-data".into(), meta_data_buf);

		// Calculate the size of volume we need
		let total_file_size: usize = files.values().map(std::vec::Vec::len).sum();
		let metadata_size = 1 << 16; // extra 64k (generously) for the FAT itself
		let volume_size = 4096 * (1 + ((total_file_size + metadata_size) / 4096)); // round up to 4k

		// Create the disk image
		let mut disk: Vec<u8> = Vec::new();
		disk.resize(volume_size, 0);

		// Format the disk
		let mut volume_label = [0u8; 11];
		volume_label.copy_from_slice("cidata     ".as_bytes());
		let format_opts = fatfs::FormatVolumeOptions::new()
			.fat_type(fatfs::FatType::Fat32)
			.volume_label(volume_label);
		fatfs::format_volume(io::Cursor::new(&mut disk), format_opts)
			.context("failed to create FAT32 disk image")?;

		// Mount the disk
		let fs_options = fatfs::FsOptions::new();
		let fs = fatfs::FileSystem::new(io::Cursor::new(&mut disk), fs_options)
			.context("failed to mount FAT32 disk image")?;

		// Write the files into the disk image
		let root = fs.root_dir();
		for (path, contents) in files {
			let mut file = root
				.create_file(&path)
				.context(format!("failed to create: {:?}", path))?;
			io::copy(&mut io::Cursor::new(contents), &mut file)
				.context(format!("failed to write: {:?}", path))?;
		}

		// Unmount the filesystem safely
		drop(root);
		fs.unmount()
			.context("failed to unmount FAT32 disk image after creation")?;

		Ok(disk)
	}
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserData {
	pub users: Vec<User>,
	pub groups: BTreeSet<GroupName>,
	pub mounts: Vec<Mount>,
	pub write_files: Vec<WriteFile>,

	/// Run this list of commands when configuring a new instance.
	#[serde(rename = "runcmd", skip_serializing_if = "Vec::is_empty")]
	pub run_commands: Vec<Vec<String>>,

	/// Run these commands very early in every boot.
	#[serde(rename = "bootcmd", skip_serializing_if = "Vec::is_empty")]
	pub boot_commands: Vec<Vec<String>>,

	/// Install these packages.
	#[serde(rename = "packages", skip_serializing_if = "Vec::is_empty")]
	pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFile {
	/// File content, as a string.
	pub content: String,

	/// How the file is encoded as a string.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub encoding: Option<WriteFileEncoding>,

	/// Path to write the file.
	pub path: String,

	/// File owner, default `root:root`.
	pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteFileEncoding {
	#[serde(rename = "base64")]
	Base64,
	#[serde(rename = "gzip+base64")]
	GzipBase64,
}

/// A user-data mount is represented as a list of each tab-separated /etc/fstab entry.
/// See: <https://cloudinit.readthedocs.io/en/latest/topics/modules.html#mounts>
pub type Mount = Vec<String>;

/// A user-data User is created inside the guest on first boot.
/// Note: This struct does not cover all the options supported by cloud-init.
///
/// See: <https://cloudinit.readthedocs.io/en/latest/topics/modules.html#users-and-groups>
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
	/// Unix user name
	pub name: String,

	/// Unix user ID. If None, the next available uid is selected by cloud-init.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub uid: Option<u32>,

	/// Sudo entry for this user.
	/// To enbale passwordless sudo, set to [`PASSWORDLESS_SUDO`].
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sudo: Option<String>,

	/// Plain-text password for the user.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub plain_text_passwd: Option<String>,

	/// Whether to disable password login for the user. If None, disables password login.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lock_passwd: Option<bool>,

	/// A user's primary group. If None, set to a new group named after the user.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub primary_group: Option<String>,

	/// A user's other groups
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub groups: Vec<String>,

	/// A list of SSH keys to create for the user
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub ssh_authorized_keys: Vec<String>,
}

/// Unix group name
pub type GroupName = String;

/// Sudo entry to enable passwordless sudo for a user.
pub const PASSWORDLESS_SUDO: &str = "ALL=(ALL) NOPASSWD:ALL";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MetaData {
	pub instance_id: String,
	pub local_hostname: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn create_image_works() {
		let mut extras = BTreeMap::new();
		extras.insert(
			"example.txt".to_string(),
			"Hello, World!".as_bytes().to_vec(),
		);

		let init_data = InitData {
			user_data: UserData::default(),
			meta_data: MetaData {
				instance_id: "instance-id-here".into(),
				local_hostname: "hostname-here".into(),
			},
			extra_files: extras,
		};

		let _img = init_data.into_disk_image().unwrap();

		// use std::io::Write;
		// let mut f = std::fs::File::create("test.img").unwrap();
		// f.write_all(&img).unwrap();
	}
}

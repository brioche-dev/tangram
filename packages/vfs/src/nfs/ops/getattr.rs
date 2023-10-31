use crate::nfs::{
	state::NodeKind,
	types::{
		bitmap4, fattr4, fs_locations4, fsid4, nfs_fh4, nfs_ftype4, nfsace4, nfsstat4, nfstime4,
		specdata4, GETATTR4args, GETATTR4res, GETATTR4resok,
	},
	xdr, Context, Server,
};
use num::ToPrimitive;
use std::{fmt::Debug, path::Path};

pub struct FileAttrData {
	pub(crate) supported_attrs: bitmap4,
	pub(crate) file_type: nfs_ftype4,
	pub(crate) expire_type: u32, // Defines how file expiry is supposed to be handled. A value of "0" is called FH4_PERSISTENT, which implies the file handle is persistent for the lifetime of the server.
	pub(crate) change: u64, // Defines how file changes happen. Since we don't have changes, we don't care.
	pub(crate) size: u64,
	pub(crate) link_support: bool, // TRUE if the file system this object is on supports hard links.
	pub(crate) symlink_support: bool, // TRUE if the file system this object is on supports soft links.
	pub(crate) named_attr: bool, // Whether this file has any nammed attributes (xattrs). TODO: care about this.
	pub(crate) fsid: fsid4, // Identifies which file system the object is on (servers may overlay multiple file systems and report such to the client).
	pub(crate) unique_handles: bool, // TRUE, if two distinct filehandles are guaranteed to refer to two different file system objects.
	pub(crate) lease_time: u32,      // The amount of time this file is valid for, in seconds.
	pub(crate) rdattr_error: i32,    // An error, if we want to return one.
	pub(crate) file_handle: nfs_fh4, // The underlying file handle
	pub(crate) acl: Vec<nfsace4>,
	pub(crate) aclsupport: u32,
	pub(crate) archive: bool,
	pub(crate) cansettime: bool,
	pub(crate) case_insensitive: bool,
	pub(crate) case_preserving: bool,
	pub(crate) chown_restricted: bool,
	pub(crate) fileid: u64,
	pub(crate) files_avail: u64,
	pub(crate) files_free: u64,
	pub(crate) files_total: u64,
	pub(crate) fs_locations: fs_locations4,
	pub(crate) hidden: bool,
	pub(crate) homogeneous: bool,
	pub(crate) maxfilesize: u64,
	pub(crate) maxlink: u32,
	pub(crate) maxname: u32,
	pub(crate) maxread: u64,
	pub(crate) maxwrite: u64,
	pub(crate) mimetype: Vec<String>,
	pub(crate) mode: u32,
	pub(crate) no_trunc: bool,
	pub(crate) numlinks: u32,
	pub(crate) owner: String,
	pub(crate) owner_group: String,
	pub(crate) quota_avail_hard: u64,
	pub(crate) quota_avail_soft: u64,
	pub(crate) quota_used: u64,
	pub(crate) rawdev: specdata4,
	pub(crate) space_avail: u64,
	pub(crate) space_free: u64,
	pub(crate) space_total: u64,
	pub(crate) space_used: u64,
	pub(crate) system: bool,
	pub(crate) time_access: nfstime4,
	pub(crate) time_backup: nfstime4,
	pub(crate) time_create: nfstime4,
	pub(crate) time_delta: nfstime4,
	pub(crate) time_metadata: nfstime4,
	pub(crate) time_modify: nfstime4,
	pub(crate) mounted_on_fileid: u64,
}

impl nfstime4 {
	fn new() -> nfstime4 {
		nfstime4 {
			seconds: 0,
			nseconds: 0,
		}
	}

	fn now() -> nfstime4 {
		todo!()
	}
}

impl FileAttrData {
	fn new(file_handle: nfs_fh4, file_type: nfs_ftype4, size: usize, mode: u32) -> FileAttrData {
		let size = size.to_u64().unwrap();
		let mut supported_attrs = Vec::new();
		for attr in ALL_SUPPORTED_ATTRS {
			// supported_attrs.set(attr.to_usize().unwrap());
		}
		let change = nfstime4::now().seconds as u64;
		FileAttrData {
			supported_attrs,
			file_type,
			expire_type: 0,
			change,
			size,
			link_support: true,
			symlink_support: true,
			named_attr: false,
			fsid: fsid4 { major: 0, minor: 1 },
			unique_handles: true,
			lease_time: 1000,
			rdattr_error: 0,
			file_handle,
			acl: vec![],
			aclsupport: 0,
			archive: true,
			cansettime: false,
			case_insensitive: false,
			case_preserving: true,
			chown_restricted: true,
			fileid: file_handle,
			files_avail: 0,
			files_free: 0,
			files_total: 1,
			hidden: false,
			homogeneous: true,
			maxfilesize: u64::MAX,
			maxlink: u32::MAX,
			maxname: 512,
			maxread: u64::MAX,
			maxwrite: 0,
			mimetype: vec![],
			mode,
			fs_locations: fs_locations4 {
				fs_root: vec!["/".as_bytes().to_owned()],
				locations: vec![],
			},
			no_trunc: true,
			numlinks: 1,
			owner: "tangram@tangram".to_owned(),
			owner_group: "tangram@tangram".to_owned(),
			quota_avail_hard: 0,
			quota_avail_soft: 0,
			quota_used: 0,
			rawdev: specdata4 {
				specdata1: 0,
				specdata2: 0,
			},
			space_avail: 0,
			space_free: 0,
			space_total: u64::MAX,
			space_used: size.to_u64().unwrap(),
			system: false,
			time_access: nfstime4::new(),
			time_backup: nfstime4::new(),
			time_create: nfstime4::new(),
			time_delta: nfstime4::new(),
			time_metadata: nfstime4::new(),
			time_modify: nfstime4::new(),
			mounted_on_fileid: file_handle,
		}
	}
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_getattr(&self, ctx: &Context, arg: GETATTR4args) -> GETATTR4res {
		let Some(fh) = ctx.current_file_handle else {
			tracing::error!("Missing current file handle.");
			return GETATTR4res::Default(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		match self.get_attr(fh, arg.attr_request).await {
			Ok(obj_attributes) => GETATTR4res::NFS4_OK(GETATTR4resok { obj_attributes }),
			Err(e) => GETATTR4res::Default(e),
		}
	}

	pub async fn get_attr(
		&self,
		file_handle: nfs_fh4,
		requested: bitmap4,
	) -> Result<fattr4, nfsstat4> {
		if requested.is_empty() {
			todo!()
			// return Ok(FileAttr {
			// 	attr_mask: Bitmap(Vec::default()),
			// 	attr_vals: Vec::new(),
			// });
		}

		let Some(data) = self.get_file_attr_data(file_handle).await else {
			tracing::error!(?file_handle, "Missing attr data.");
			return Err(nfsstat4::NFS4ERR_NOENT);
		};

		todo!()
		// let attr_mask = data.supported_attrs.intersection(&requested);
		// let attr_vals = data.to_bytes(&attr_mask);

		// Ok(FileAttr {
		// 	attr_mask,
		// 	attr_vals,
		// })
	}

	pub async fn get_file_attr_data(&self, file_handle: nfs_fh4) -> Option<FileAttrData> {
		let state = &self.state.read().await;
		let node = state.nodes.get(&file_handle)?;
		let data = match &node.kind {
			NodeKind::Root { .. } => FileAttrData::new(file_handle, nfs_ftype4::NF4DIR, 0, O_RX),
			NodeKind::Directory { children, .. } => {
				let len = children.read().await.len();
				FileAttrData::new(file_handle, nfs_ftype4::NF4DIR, len, O_RX)
			},
			NodeKind::File { file, size } => {
				let is_executable = match file.executable(self.client.as_ref()).await {
					Ok(b) => b,
					Err(e) => {
						tracing::error!(?e, "Failed to lookup executable bit for file.");
						return None;
					},
				};
				let mode = if is_executable { O_RX } else { O_RDONLY };

				FileAttrData::new(
					file_handle,
					nfs_ftype4::NF4REG,
					size.to_usize().unwrap(),
					mode,
				)
			},
			NodeKind::Symlink { .. } => {
				// TODO: size of symlink
				FileAttrData::new(file_handle, nfs_ftype4::NF4LNK, 1, O_RDONLY)
			},
		};
		Some(data)
	}
}

use consts::*;

#[allow(dead_code)]
pub mod consts {
	// Flags for mode
	pub const MODE4_SUID: u32 = 0x800; /* set user id on execution */
	pub const MODE4_SGID: u32 = 0x400; /* set group id on execution */
	pub const MODE4_SVTX: u32 = 0x200; /* save text even after use */
	pub const MODE4_RUSR: u32 = 0x100; /* read permission: owner */
	pub const MODE4_WUSR: u32 = 0x080; /* write permission: owner */
	pub const MODE4_XUSR: u32 = 0x040; /* execute permission: owner */
	pub const MODE4_RGRP: u32 = 0x020; /* read permission: group */
	pub const MODE4_WGRP: u32 = 0x010; /* write permission: group */
	pub const MODE4_XGRP: u32 = 0x008; /* execute permission: group */
	pub const O_RDONLY: u32 = MODE4_RUSR | MODE4_RGRP;
	pub const O_RX: u32 = MODE4_XUSR | MODE4_XGRP | O_RDONLY;

	// Attribute numbers.
	pub const SUPPORTED_ATTRS: u32 = 0; // Required
	pub const TYPE: u32 = 1; // Required
	pub const FH_EXPIRE_TYPE: u32 = 2; // Required
	pub const CHANGE: u32 = 3; // Required
	pub const SIZE: u32 = 4; // Required
	pub const LINK_SUPPORT: u32 = 5; // Required
	pub const SYMLINK_SUPPORT: u32 = 6; // Required
	pub const NAMED_ATTR: u32 = 7; // Required
	pub const FSID: u32 = 8; // Required
	pub const UNIQUE_HANDLES: u32 = 9; // Required
	pub const LEASE_TIME: u32 = 10; // Required
	pub const RDATTR_ERROR: u32 = 11; // Required
	pub const ACL: u32 = 12;
	pub const ACLSUPPORT: u32 = 13;
	pub const ARCHIVE: u32 = 14;
	pub const CANSETTIME: u32 = 15;
	pub const CASE_INSENSITIVE: u32 = 16;
	pub const CASE_PRESERVING: u32 = 17;
	pub const CHOWN_RESTRICTED: u32 = 18;
	pub const FILEHANDLE: u32 = 19; // Required
	pub const FILEID: u32 = 20;
	pub const FILES_AVAIL: u32 = 21;
	pub const FILES_FREE: u32 = 22;
	pub const FILES_TOTAL: u32 = 23;
	pub const FS_LOCATIONS: u32 = 24;
	pub const HIDDEN: u32 = 25;
	pub const HOMOGENEOUS: u32 = 26;
	pub const MAXFILESIZE: u32 = 27;
	pub const MAXLINK: u32 = 28;
	pub const MAXNAME: u32 = 29;
	pub const MAXREAD: u32 = 30;
	pub const MAXWRITE: u32 = 31;
	pub const MIMETYPE: u32 = 32;
	pub const MODE: u32 = 33;
	pub const NO_TRUNC: u32 = 34;
	pub const NUMLINKS: u32 = 35;
	pub const OWNER: u32 = 36;
	pub const OWNER_GROUP: u32 = 37;
	pub const QUOTA_AVAIL_HARD: u32 = 38;
	pub const QUOTA_AVAIL_SOFT: u32 = 39;
	pub const QUOTA_USED: u32 = 40;
	pub const RAWDEV: u32 = 41;
	pub const SPACE_AVAIL: u32 = 42;
	pub const SPACE_FREE: u32 = 43;
	pub const SPACE_TOTAL: u32 = 44;
	pub const SPACE_USED: u32 = 45;
	pub const SYSTEM: u32 = 46;
	pub const TIME_ACCESS: u32 = 47;
	pub const TIME_ACCESS_SET: u32 = 48;
	pub const TIME_BACKUP: u32 = 49;
	pub const TIME_CREATE: u32 = 50;
	pub const TIME_DELTA: u32 = 51;
	pub const TIME_METADATA: u32 = 52;
	pub const TIME_MODIFY: u32 = 53;
	pub const TIME_MODIFY_SET: u32 = 54;
	pub const MOUNTED_ON_FILEID: u32 = 55;
	pub const DIR_NOTIF_DELAY: u32 = 56;
	pub const DIRENT_NOTIF_DELAY: u32 = 57;
	pub const DACL: u32 = 58;
	pub const SACL: u32 = 59;
	pub const CHANGE_POLICY: u32 = 60;
	pub const FS_STATUS: u32 = 61;
	pub const FS_LAYOUT_TYPE: u32 = 62;
	pub const LAYOUT_HINT: u32 = 63;
	pub const LAYOUT_TYPE: u32 = 64;
	pub const LAYOUT_BLKSIZE: u32 = 65;
	pub const LAYOUT_ALIGNMENT: u32 = 66;
	pub const FS_LOCATIONS_INFO: u32 = 67;
	pub const MDSTHRESHOLD: u32 = 68;
	pub const RETENTION_GET: u32 = 69;
	pub const RETENTION_SET: u32 = 70;
	pub const RETENTEVT_GET: u32 = 71;
	pub const RETENTEVT_SET: u32 = 72;
	pub const RETENTION_HOLD: u32 = 73;
	pub const MODE_SET_MASKED: u32 = 74;
	pub const SUPPATTR_EXCLCREAT: u32 = 75; // Required
	pub const FS_CHARSET_CAP: u32 = 76;

	// List of all attributes.
	pub const ALL_ATTRS: [u32; 77] = [
		SUPPORTED_ATTRS,
		TYPE,
		FH_EXPIRE_TYPE,
		CHANGE,
		SIZE,
		LINK_SUPPORT,
		SYMLINK_SUPPORT,
		NAMED_ATTR,
		FSID,
		UNIQUE_HANDLES,
		LEASE_TIME,
		RDATTR_ERROR,
		ACL,
		ACLSUPPORT,
		ARCHIVE,
		CANSETTIME,
		CASE_INSENSITIVE,
		CASE_PRESERVING,
		CHOWN_RESTRICTED,
		FILEHANDLE,
		FILEID,
		FILES_AVAIL,
		FILES_FREE,
		FILES_TOTAL,
		FS_LOCATIONS,
		HIDDEN,
		HOMOGENEOUS,
		MAXFILESIZE,
		MAXLINK,
		MAXNAME,
		MAXREAD,
		MAXWRITE,
		MIMETYPE,
		MODE,
		NO_TRUNC,
		NUMLINKS,
		OWNER,
		OWNER_GROUP,
		QUOTA_AVAIL_HARD,
		QUOTA_AVAIL_SOFT,
		QUOTA_USED,
		RAWDEV,
		SPACE_AVAIL,
		SPACE_FREE,
		SPACE_TOTAL,
		SPACE_USED,
		SYSTEM,
		TIME_ACCESS,
		TIME_ACCESS_SET,
		TIME_BACKUP,
		TIME_CREATE,
		TIME_DELTA,
		TIME_METADATA,
		TIME_MODIFY,
		TIME_MODIFY_SET,
		MOUNTED_ON_FILEID,
		DIR_NOTIF_DELAY,
		DIRENT_NOTIF_DELAY,
		DACL,
		SACL,
		CHANGE_POLICY,
		FS_STATUS,
		FS_LAYOUT_TYPE,
		LAYOUT_HINT,
		LAYOUT_TYPE,
		LAYOUT_BLKSIZE,
		LAYOUT_ALIGNMENT,
		FS_LOCATIONS_INFO,
		MDSTHRESHOLD,
		RETENTION_GET,
		RETENTION_SET,
		RETENTEVT_GET,
		RETENTEVT_SET,
		RETENTION_HOLD,
		MODE_SET_MASKED,
		SUPPATTR_EXCLCREAT,
		FS_CHARSET_CAP,
	];

	pub const ALL_SUPPORTED_ATTRS: &[u32] = &[
		SUPPORTED_ATTRS,
		TYPE,
		FH_EXPIRE_TYPE,
		CHANGE,
		SIZE,
		LINK_SUPPORT,
		SYMLINK_SUPPORT,
		NAMED_ATTR,
		FSID,
		UNIQUE_HANDLES,
		LEASE_TIME,
		RDATTR_ERROR,
		ARCHIVE,
		CANSETTIME,
		CASE_INSENSITIVE,
		CASE_PRESERVING,
		CHOWN_RESTRICTED,
		FILEHANDLE,
		FILEID,
		FILES_AVAIL,
		FILES_FREE,
		FILES_TOTAL,
		FS_LOCATIONS,
		HIDDEN,
		HOMOGENEOUS,
		MAXFILESIZE,
		MAXLINK,
		MAXNAME,
		MAXREAD,
		MAXWRITE,
		MIMETYPE,
		MODE,
		NO_TRUNC,
		NUMLINKS,
		OWNER,
		OWNER_GROUP,
		QUOTA_AVAIL_HARD,
		QUOTA_AVAIL_SOFT,
		QUOTA_USED,
		RAWDEV,
		SPACE_AVAIL,
		SPACE_FREE,
		SPACE_TOTAL,
		SPACE_USED,
		SYSTEM,
		TIME_ACCESS,
		TIME_BACKUP,
		TIME_CREATE,
		TIME_DELTA,
		TIME_METADATA,
		TIME_MODIFY,
		MOUNTED_ON_FILEID,
	];

	pub const ACE4_ACCESS_ALLOWED_ACE_TYPE: u32 = 0x00000000;
	pub const ACE4_ACCESS_DENIED_ACE_TYPE: u32 = 0x00000001;
	pub const ACE4_SYSTEM_AUDIT_ACE_TYPE: u32 = 0x00000002;
	pub const ACE4_SYSTEM_ALARM_ACE_TYPE: u32 = 0x00000003;
	pub const ACL4_SUPPORT_ALLOW_ACL: u32 = 0x00000001;
	pub const ACL4_SUPPORT_DENY_ACL: u32 = 0x00000002;
	pub const ACL4_SUPPORT_AUDIT_ACL: u32 = 0x00000004;
	pub const ACL4_SUPPORT_ALARM_ACL: u32 = 0x00000008;
}

#[derive(Debug, Clone)]
struct Locations {
	fs_root: Pathname,
	locations: Vec<Location>,
}

impl xdr::ToXdr for Locations {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.fs_root)?;
		encoder.encode(&self.locations)?;
		Ok(())
	}
}

#[derive(Debug, Clone)]
struct Location {
	server: Pathname,
	rootpath: Vec<String>,
}

impl xdr::ToXdr for Location {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.server)?;
		encoder.encode(&self.rootpath)?;
		Ok(())
	}
}

#[derive(Debug, Clone)]
struct Pathname {
	components: Vec<String>,
}

impl<P> From<P> for Pathname
where
	P: AsRef<Path>,
{
	fn from(value: P) -> Self {
		let components = value
			.as_ref()
			.components()
			.map(|component| component.as_os_str().to_str().unwrap().to_owned())
			.collect();
		Self { components }
	}
}

impl xdr::ToXdr for Pathname {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.components)
	}
}

impl FileAttrData {
	fn to_bytes(&self, requested: &bitmap4) -> Vec<u8> {
		let mut buf = Vec::with_capacity(256);
		let mut encoder = xdr::Encoder::new(&mut buf);
		for attr in ALL_SUPPORTED_ATTRS.iter().copied() {
			// if !requested.get(attr.to_usize().unwrap()) {
			// 	continue;
			// }
			match attr {
				SUPPORTED_ATTRS => encoder.encode(&self.supported_attrs).unwrap(),
				TYPE => encoder.encode(&self.file_type).unwrap(),
				FH_EXPIRE_TYPE => encoder.encode(&self.expire_type).unwrap(),
				CHANGE => encoder.encode(&self.change).unwrap(),
				SIZE => encoder.encode(&self.size).unwrap(),
				LINK_SUPPORT => encoder.encode(&self.link_support).unwrap(),
				SYMLINK_SUPPORT => encoder.encode(&self.symlink_support).unwrap(),
				NAMED_ATTR => encoder.encode(&self.named_attr).unwrap(),
				FSID => encoder.encode(&self.fsid).unwrap(),
				UNIQUE_HANDLES => encoder.encode(&self.unique_handles).unwrap(),
				LEASE_TIME => encoder.encode(&self.lease_time).unwrap(),
				RDATTR_ERROR => encoder.encode(&self.rdattr_error).unwrap(),
				FILEHANDLE => encoder.encode(&self.file_handle).unwrap(),
				ACL => encoder.encode(&self.acl).unwrap(),
				ACLSUPPORT => encoder.encode(&self.aclsupport).unwrap(),
				ARCHIVE => encoder.encode(&self.archive).unwrap(),
				CANSETTIME => encoder.encode(&self.cansettime).unwrap(),
				CASE_INSENSITIVE => encoder.encode(&self.case_insensitive).unwrap(),
				CASE_PRESERVING => encoder.encode(&self.case_preserving).unwrap(),
				CHOWN_RESTRICTED => encoder.encode(&self.chown_restricted).unwrap(),
				FILEID => encoder.encode(&self.fileid).unwrap(),
				FILES_AVAIL => encoder.encode(&self.files_avail).unwrap(),
				FILES_FREE => encoder.encode(&self.files_free).unwrap(),
				FILES_TOTAL => encoder.encode(&self.files_total).unwrap(),
				HIDDEN => encoder.encode(&self.hidden).unwrap(),
				HOMOGENEOUS => encoder.encode(&self.homogeneous).unwrap(),
				MAXFILESIZE => encoder.encode(&self.maxfilesize).unwrap(),
				MAXLINK => encoder.encode(&self.maxlink).unwrap(),
				MAXNAME => encoder.encode(&self.maxname).unwrap(),
				MAXREAD => encoder.encode(&self.maxread).unwrap(),
				MAXWRITE => encoder.encode(&self.maxwrite).unwrap(),
				MIMETYPE => encoder.encode(&self.mimetype).unwrap(),
				MODE => encoder.encode(&self.mode).unwrap(),
				FS_LOCATIONS => encoder.encode(&self.fs_locations).unwrap(),
				NO_TRUNC => encoder.encode(&self.no_trunc).unwrap(),
				NUMLINKS => encoder.encode(&self.numlinks).unwrap(),
				OWNER => encoder.encode(&self.owner).unwrap(),
				OWNER_GROUP => encoder.encode(&self.owner_group).unwrap(),
				QUOTA_AVAIL_HARD => encoder.encode(&self.quota_avail_hard).unwrap(),
				QUOTA_AVAIL_SOFT => encoder.encode(&self.quota_avail_soft).unwrap(),
				QUOTA_USED => encoder.encode(&self.quota_used).unwrap(),
				RAWDEV => encoder.encode(&self.rawdev).unwrap(),
				SPACE_AVAIL => encoder.encode(&self.space_avail).unwrap(),
				SPACE_FREE => encoder.encode(&self.space_free).unwrap(),
				SPACE_TOTAL => encoder.encode(&self.space_total).unwrap(),
				SPACE_USED => encoder.encode(&self.space_used).unwrap(),
				SYSTEM => encoder.encode(&self.system).unwrap(),
				TIME_ACCESS => encoder.encode(&self.time_access).unwrap(),
				// TIME_ACCESS_SET => encoder.encode(&self.time_access_set).unwrap(),
				TIME_BACKUP => encoder.encode(&self.time_backup).unwrap(),
				TIME_CREATE => encoder.encode(&self.time_create).unwrap(),
				TIME_DELTA => encoder.encode(&self.time_delta).unwrap(),
				TIME_METADATA => encoder.encode(&self.time_metadata).unwrap(),
				TIME_MODIFY => encoder.encode(&self.time_modify).unwrap(),
				// TIME_MODIFY_SET => encoder.encode(&self.time_modify_set).unwrap(),
				MOUNTED_ON_FILEID => encoder.encode(&self.mounted_on_fileid).unwrap(),
				_ => (),
			};
		}

		buf
	}
}

use crate::nfs::{
	state::NodeKind,
	types::{
		bitmap4, fattr4, fs_locations4, fsid4, nfs_fh4, nfs_ftype4, nfsace4, nfsstat4, nfstime4,
		pathname4, specdata4, GETATTR4args, GETATTR4res, GETATTR4resok, FATTR4_ACL,
		FATTR4_ACLSUPPORT, FATTR4_ARCHIVE, FATTR4_CANSETTIME, FATTR4_CASE_INSENSITIVE,
		FATTR4_CASE_PRESERVING, FATTR4_CHANGE, FATTR4_CHOWN_RESTRICTED, FATTR4_FH_EXPIRE_TYPE,
		FATTR4_FILEHANDLE, FATTR4_FILEID, FATTR4_FILES_AVAIL, FATTR4_FILES_FREE,
		FATTR4_FILES_TOTAL, FATTR4_FSID, FATTR4_FS_LOCATIONS, FATTR4_HIDDEN, FATTR4_HOMOGENEOUS,
		FATTR4_LEASE_TIME, FATTR4_LINK_SUPPORT, FATTR4_MAXFILESIZE, FATTR4_MAXLINK, FATTR4_MAXNAME,
		FATTR4_MAXREAD, FATTR4_MAXWRITE, FATTR4_MIMETYPE, FATTR4_MODE, FATTR4_MOUNTED_ON_FILEID,
		FATTR4_NAMED_ATTR, FATTR4_NO_TRUNC, FATTR4_NUMLINKS, FATTR4_OWNER, FATTR4_OWNER_GROUP,
		FATTR4_QUOTA_AVAIL_HARD, FATTR4_QUOTA_AVAIL_SOFT, FATTR4_QUOTA_USED, FATTR4_RAWDEV,
		FATTR4_RDATTR_ERROR, FATTR4_SIZE, FATTR4_SPACE_AVAIL, FATTR4_SPACE_FREE,
		FATTR4_SPACE_TOTAL, FATTR4_SPACE_USED, FATTR4_SUPPORTED_ATTRS, FATTR4_SYMLINK_SUPPORT,
		FATTR4_SYSTEM, FATTR4_TIME_ACCESS, FATTR4_TIME_BACKUP, FATTR4_TIME_CREATE,
		FATTR4_TIME_DELTA, FATTR4_TIME_METADATA, FATTR4_TIME_MODIFY, FATTR4_TYPE,
		FATTR4_UNIQUE_HANDLES, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR, MODE4_XGRP, MODE4_XOTH,
		MODE4_XUSR,
	},
	xdr, Context, Server,
};
use num::ToPrimitive;

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

impl FileAttrData {
	fn new(file_handle: nfs_fh4, file_type: nfs_ftype4, size: usize, mode: u32) -> FileAttrData {
		let size = size.to_u64().unwrap();
		let mut supported_attrs = bitmap4(Vec::new());
		for attr in ALL_SUPPORTED_ATTRS {
			supported_attrs.set(attr.to_usize().unwrap());
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
			acl: Vec::new(),
			aclsupport: 0,
			archive: true,
			cansettime: false,
			case_insensitive: false,
			case_preserving: true,
			chown_restricted: true,
			fileid: file_handle.0,
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
			mimetype: Vec::new(),
			mode,
			fs_locations: fs_locations4 {
				fs_root: pathname4(vec!["/".as_bytes().to_owned()]),
				locations: Vec::new(),
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
			mounted_on_fileid: file_handle.0,
		}
	}
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_getattr(&self, ctx: &Context, arg: GETATTR4args) -> GETATTR4res {
		let Some(fh) = ctx.current_file_handle else {
			tracing::error!("Missing current file handle.");
			return GETATTR4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		match self.get_attr(fh, arg.attr_request).await {
			Ok(obj_attributes) => GETATTR4res::NFS4_OK(GETATTR4resok { obj_attributes }),
			Err(e) => GETATTR4res::Error(e),
		}
	}

	pub async fn get_attr(
		&self,
		file_handle: nfs_fh4,
		requested: bitmap4,
	) -> Result<fattr4, nfsstat4> {
		if requested.0.is_empty() {
			return Ok(fattr4 {
				attrmask: bitmap4(Vec::default()),
				attr_vals: Vec::new(),
			});
		}

		let Some(data) = self.get_file_attr_data(file_handle).await else {
			tracing::error!(?file_handle, "Missing attr data.");
			return Err(nfsstat4::NFS4ERR_NOENT);
		};

		let attrmask = data.supported_attrs.intersection(&requested);
		let attr_vals = data.to_bytes(&attrmask);

		Ok(fattr4 {
			attrmask,
			attr_vals,
		})
	}

	pub async fn get_file_attr_data(&self, file_handle: nfs_fh4) -> Option<FileAttrData> {
		let state = &self.state.read().await;
		let node = state.nodes.get(&file_handle.0)?;
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

// use consts::*;

pub const O_RDONLY: u32 = MODE4_RUSR | MODE4_RGRP | MODE4_ROTH;
pub const O_RX: u32 = MODE4_XUSR | MODE4_XGRP | MODE4_XOTH | O_RDONLY;

pub const ALL_SUPPORTED_ATTRS: &[u32] = &[
	FATTR4_SUPPORTED_ATTRS,
	FATTR4_TYPE,
	FATTR4_FH_EXPIRE_TYPE,
	FATTR4_CHANGE,
	FATTR4_SIZE,
	FATTR4_LINK_SUPPORT,
	FATTR4_SYMLINK_SUPPORT,
	FATTR4_NAMED_ATTR,
	FATTR4_FSID,
	FATTR4_UNIQUE_HANDLES,
	FATTR4_LEASE_TIME,
	FATTR4_RDATTR_ERROR,
	FATTR4_ARCHIVE,
	FATTR4_CANSETTIME,
	FATTR4_CASE_INSENSITIVE,
	FATTR4_CASE_PRESERVING,
	FATTR4_CHOWN_RESTRICTED,
	FATTR4_FILEHANDLE,
	FATTR4_FILEID,
	FATTR4_FILES_AVAIL,
	FATTR4_FILES_FREE,
	FATTR4_FILES_TOTAL,
	FATTR4_FS_LOCATIONS,
	FATTR4_HIDDEN,
	FATTR4_HOMOGENEOUS,
	FATTR4_MAXFILESIZE,
	FATTR4_MAXLINK,
	FATTR4_MAXNAME,
	FATTR4_MAXREAD,
	FATTR4_MAXWRITE,
	FATTR4_MIMETYPE,
	FATTR4_MODE,
	FATTR4_NO_TRUNC,
	FATTR4_NUMLINKS,
	FATTR4_OWNER,
	FATTR4_OWNER_GROUP,
	FATTR4_QUOTA_AVAIL_HARD,
	FATTR4_QUOTA_AVAIL_SOFT,
	FATTR4_QUOTA_USED,
	FATTR4_RAWDEV,
	FATTR4_SPACE_AVAIL,
	FATTR4_SPACE_FREE,
	FATTR4_SPACE_TOTAL,
	FATTR4_SPACE_USED,
	FATTR4_SYSTEM,
	FATTR4_TIME_ACCESS,
	FATTR4_TIME_BACKUP,
	FATTR4_TIME_CREATE,
	FATTR4_TIME_DELTA,
	FATTR4_TIME_METADATA,
	FATTR4_TIME_MODIFY,
	FATTR4_MOUNTED_ON_FILEID,
];

impl FileAttrData {
	fn to_bytes(&self, requested: &bitmap4) -> Vec<u8> {
		let mut buf = Vec::with_capacity(256);
		let mut encoder = xdr::Encoder::new(&mut buf);
		for attr in ALL_SUPPORTED_ATTRS.iter().copied() {
			if !requested.get(attr.to_usize().unwrap()) {
				continue;
			}
			match attr {
				FATTR4_SUPPORTED_ATTRS => encoder.encode(&self.supported_attrs.0).unwrap(),
				FATTR4_TYPE => encoder.encode(&self.file_type).unwrap(),
				FATTR4_FH_EXPIRE_TYPE => encoder.encode(&self.expire_type).unwrap(),
				FATTR4_CHANGE => encoder.encode(&self.change).unwrap(),
				FATTR4_SIZE => encoder.encode(&self.size).unwrap(),
				FATTR4_LINK_SUPPORT => encoder.encode(&self.link_support).unwrap(),
				FATTR4_SYMLINK_SUPPORT => encoder.encode(&self.symlink_support).unwrap(),
				FATTR4_NAMED_ATTR => encoder.encode(&self.named_attr).unwrap(),
				FATTR4_FSID => encoder.encode(&self.fsid).unwrap(),
				FATTR4_UNIQUE_HANDLES => encoder.encode(&self.unique_handles).unwrap(),
				FATTR4_LEASE_TIME => encoder.encode(&self.lease_time).unwrap(),
				FATTR4_RDATTR_ERROR => encoder.encode(&self.rdattr_error).unwrap(),
				FATTR4_FILEHANDLE => encoder.encode(&self.file_handle).unwrap(),
				FATTR4_ACL => encoder.encode(&self.acl).unwrap(),
				FATTR4_ACLSUPPORT => encoder.encode(&self.aclsupport).unwrap(),
				FATTR4_ARCHIVE => encoder.encode(&self.archive).unwrap(),
				FATTR4_CANSETTIME => encoder.encode(&self.cansettime).unwrap(),
				FATTR4_CASE_INSENSITIVE => encoder.encode(&self.case_insensitive).unwrap(),
				FATTR4_CASE_PRESERVING => encoder.encode(&self.case_preserving).unwrap(),
				FATTR4_CHOWN_RESTRICTED => encoder.encode(&self.chown_restricted).unwrap(),
				FATTR4_FILEID => encoder.encode(&self.fileid).unwrap(),
				FATTR4_FILES_AVAIL => encoder.encode(&self.files_avail).unwrap(),
				FATTR4_FILES_FREE => encoder.encode(&self.files_free).unwrap(),
				FATTR4_FILES_TOTAL => encoder.encode(&self.files_total).unwrap(),
				FATTR4_HIDDEN => encoder.encode(&self.hidden).unwrap(),
				FATTR4_HOMOGENEOUS => encoder.encode(&self.homogeneous).unwrap(),
				FATTR4_MAXFILESIZE => encoder.encode(&self.maxfilesize).unwrap(),
				FATTR4_MAXLINK => encoder.encode(&self.maxlink).unwrap(),
				FATTR4_MAXNAME => encoder.encode(&self.maxname).unwrap(),
				FATTR4_MAXREAD => encoder.encode(&self.maxread).unwrap(),
				FATTR4_MAXWRITE => encoder.encode(&self.maxwrite).unwrap(),
				FATTR4_MIMETYPE => encoder.encode(&self.mimetype).unwrap(),
				FATTR4_MODE => encoder.encode(&self.mode).unwrap(),
				FATTR4_FS_LOCATIONS => encoder.encode(&self.fs_locations).unwrap(),
				FATTR4_NO_TRUNC => encoder.encode(&self.no_trunc).unwrap(),
				FATTR4_NUMLINKS => encoder.encode(&self.numlinks).unwrap(),
				FATTR4_OWNER => encoder.encode(&self.owner).unwrap(),
				FATTR4_OWNER_GROUP => encoder.encode(&self.owner_group).unwrap(),
				FATTR4_QUOTA_AVAIL_HARD => encoder.encode(&self.quota_avail_hard).unwrap(),
				FATTR4_QUOTA_AVAIL_SOFT => encoder.encode(&self.quota_avail_soft).unwrap(),
				FATTR4_QUOTA_USED => encoder.encode(&self.quota_used).unwrap(),
				FATTR4_RAWDEV => encoder.encode(&self.rawdev).unwrap(),
				FATTR4_SPACE_AVAIL => encoder.encode(&self.space_avail).unwrap(),
				FATTR4_SPACE_FREE => encoder.encode(&self.space_free).unwrap(),
				FATTR4_SPACE_TOTAL => encoder.encode(&self.space_total).unwrap(),
				FATTR4_SPACE_USED => encoder.encode(&self.space_used).unwrap(),
				FATTR4_SYSTEM => encoder.encode(&self.system).unwrap(),
				FATTR4_TIME_ACCESS => encoder.encode(&self.time_access).unwrap(),
				FATTR4_TIME_BACKUP => encoder.encode(&self.time_backup).unwrap(),
				FATTR4_TIME_CREATE => encoder.encode(&self.time_create).unwrap(),
				FATTR4_TIME_DELTA => encoder.encode(&self.time_delta).unwrap(),
				FATTR4_TIME_METADATA => encoder.encode(&self.time_metadata).unwrap(),
				FATTR4_TIME_MODIFY => encoder.encode(&self.time_modify).unwrap(),
				FATTR4_MOUNTED_ON_FILEID => encoder.encode(&self.mounted_on_fileid).unwrap(),
				_ => (),
			};
		}

		buf
	}
}

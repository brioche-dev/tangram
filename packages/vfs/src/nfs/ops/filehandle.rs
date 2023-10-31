use crate::nfs::{
	types::{nfs_fh4, nfsstat4},
	Context,
};

pub fn put(ctx: &mut Context, arg: nfs_fh4) {
	ctx.current_file_handle = Some(arg);
}

pub fn get(ctx: &Context) -> Result<nfs_fh4, nfsstat4> {
	ctx.current_file_handle.ok_or(nfsstat4::NFS4ERR_BADHANDLE)
}

pub fn save(ctx: &mut Context) {
	ctx.saved_file_handle = ctx.current_file_handle;
}

pub fn restore(ctx: &mut Context) {
	ctx.current_file_handle = ctx.saved_file_handle.take();
}
